// Client for Tetra's local session API (`/run/tetra/tetra.sock`).
//
// Tetra owns the first-boot setup code. The OOBE asks the user for it, redeems
// it with `setup.redeem` for a setup-only session token, and sends that token
// with every request. Tetra checks the token and the setup session's allowed
// actions on every request; the OOBE never decides on its own what a caller may
// do. See Tetra's docs/auth-protocol.md and docs/dashboard-auth-guide.md.
//
// This module must not import SvelteKit aliases ($lib, $app, $env) so that
// tests can load it directly.

import { env } from 'node:process';
import { randomUUID } from 'node:crypto';
import WebSocket from 'ws';
import { MockTetraClient } from './tetra-mock';
import type { WifiSecurityProfile } from '../oobe-state';
import type { NetworkConfigureRequest } from '../network-config';
import {
	DEFAULT_TETRA_SOCKET,
	type AccessRequest,
	type AccessResult,
	type CloudflaredTokenRequest,
	type RemoteAccessStatus,
	type SetupCompleteResult,
	type TailscaleUpRequest,
	type UpdatePolicyRequest,
	TETRA_PROTOCOL_VERSION,
	TetraError,
	type SessionInfo,
	type SetupSession,
	type SetupStatus,
	type TetraClient,
	type TetraErrorCode
} from './tetra-api';

export * from './tetra-api';

interface ResponseFrame {
	type: 'response';
	id: string;
	ok: boolean;
	result?: unknown;
	error?: { code: TetraErrorCode; message: string; retry_after?: number };
}

interface HelloFrame {
	type: 'hello';
	protocol_version: string;
	client: string;
	setup_required: boolean;
	passkeys_available: boolean;
}

interface ErrorFrame {
	type: 'error';
	error: { code: TetraErrorCode; message: string };
}

type ServerFrame = ResponseFrame | HelloFrame | ErrorFrame;

const REQUEST_TIMEOUT_MS = 120_000;

/** Seconds Tetra waits before stopping the OOBE, when it does not say. */
const DEFAULT_HANDOFF_SECONDS = 5;

export function parseSetupComplete(result: Record<string, unknown>): SetupCompleteResult {
	return {
		dashboardUrl: typeof result.dashboard_url === 'string' ? result.dashboard_url : undefined,
		handoffInSeconds:
			typeof result.handoff_in_seconds === 'number'
				? result.handoff_in_seconds
				: DEFAULT_HANDOFF_SECONDS
	};
}

export function parseRemoteAccessStatus(result: Record<string, unknown>): RemoteAccessStatus {
	const tailscale = asRecord(result.tailscale);
	const cloudflared = asRecord(result.cloudflared);
	return {
		tailscale: {
			enabled: tailscale.enabled === true,
			serve: tailscale.serve === true,
			funnel: tailscale.funnel === true,
			hostname: typeof tailscale.hostname === 'string' ? tailscale.hostname : undefined
		},
		cloudflared: { enabled: cloudflared.enabled === true }
	};
}

function asRecord(value: unknown): Record<string, unknown> {
	return value && typeof value === 'object' && !Array.isArray(value)
		? (value as Record<string, unknown>)
		: {};
}

/**
 * One WebSocket connection over Tetra's Unix socket, shared by every request.
 * Requests carry their own token and are matched to responses by id.
 */
export class SocketTetraClient implements TetraClient {
	private socket: WebSocket | undefined;
	private ready: Promise<WebSocket> | undefined;
	private readonly pending = new Map<
		string,
		{ resolve: (frame: ResponseFrame) => void; reject: (error: Error) => void }
	>();

	constructor(private readonly socketPath: string = DEFAULT_TETRA_SOCKET) {}

	private connect(): Promise<WebSocket> {
		if (this.ready) return this.ready;
		this.ready = new Promise<WebSocket>((resolve, reject) => {
			const socket = new WebSocket(`ws+unix://${this.socketPath}:/`);
			let greeted = false;
			const fail = (error: TetraError) => {
				if (!greeted) reject(error);
				this.reset(error);
			};
			socket.on('message', (data) => {
				let frame: ServerFrame;
				try {
					frame = JSON.parse(data.toString()) as ServerFrame;
				} catch {
					return;
				}
				if (frame.type === 'hello') {
					if (frame.protocol_version !== TETRA_PROTOCOL_VERSION) {
						fail(
							new TetraError(
								'unavailable',
								`Unsupported Tetra protocol ${frame.protocol_version}; expected ${TETRA_PROTOCOL_VERSION}`
							)
						);
						socket.close();
						return;
					}
					greeted = true;
					this.socket = socket;
					resolve(socket);
				} else if (frame.type === 'response') {
					const waiter = this.pending.get(frame.id);
					this.pending.delete(frame.id);
					waiter?.resolve(frame);
				} else if (frame.type === 'error') {
					// Sent without an id, for example when Tetra refuses this UID.
					fail(new TetraError(frame.error.code, frame.error.message));
				}
			});
			socket.on('error', (error) =>
				fail(new TetraError('unavailable', `Cannot reach Tetra: ${error.message}`))
			);
			socket.on('close', () => fail(new TetraError('unavailable', 'Connection to Tetra closed')));
		});
		return this.ready;
	}

	private reset(error: Error) {
		this.socket = undefined;
		this.ready = undefined;
		for (const waiter of this.pending.values()) waiter.reject(error);
		this.pending.clear();
	}

	private async call(op: string, params: Record<string, unknown>, token?: string) {
		const socket = await this.connect();
		const id = randomUUID();
		const frame = await new Promise<ResponseFrame>((resolve, reject) => {
			const timer = setTimeout(() => {
				this.pending.delete(id);
				reject(new TetraError('unavailable', `Tetra did not answer ${op}`));
			}, REQUEST_TIMEOUT_MS);
			this.pending.set(id, {
				resolve: (response) => {
					clearTimeout(timer);
					resolve(response);
				},
				reject: (error) => {
					clearTimeout(timer);
					reject(error);
				}
			});
			socket.send(JSON.stringify({ type: 'request', id, token, op, params }));
		});
		if (!frame.ok) {
			const error = frame.error ?? { code: 'internal' as const, message: 'Tetra request failed' };
			throw new TetraError(error.code, error.message, error.retry_after);
		}
		return asRecord(frame.result);
	}

	async setupStatus(): Promise<SetupStatus> {
		const result = await this.call('setup.status', {});
		return {
			setupRequired: result.setup_required === true,
			passkeysAvailable: result.passkeys_available === true
		};
	}

	async redeemSetupCode(code: string, remoteAddr?: string): Promise<SetupSession> {
		const result = await this.call(
			'setup.redeem',
			remoteAddr ? { code, remote_addr: remoteAddr } : { code }
		);
		const session = asRecord(result.session);
		if (typeof result.token !== 'string' || session.kind !== 'setup') {
			throw new TetraError('unavailable', 'Tetra returned an unexpected setup session');
		}
		return {
			token: result.token,
			expiresAt: typeof session.expires_at === 'number' ? session.expires_at : 0,
			adminExists: result.admin_exists === true
		};
	}

	async sessionInfo(token: string): Promise<SessionInfo> {
		const session = asRecord((await this.call('session.info', {}, token)).session);
		return {
			kind: session.kind === 'setup' ? 'setup' : 'user',
			client: session.client === 'oobe' ? 'oobe' : 'dashboard',
			expiresAt: typeof session.expires_at === 'number' ? session.expires_at : 0
		};
	}

	async setAccess(token: string, request: AccessRequest): Promise<AccessResult> {
		const result = await this.call('access.set', { ...request }, token);
		return {
			passkeysAvailable: result.passkeys_available === true,
			dashboardUrl: typeof result.dashboard_url === 'string' ? result.dashboard_url : undefined
		};
	}

	async completeSetup(token: string): Promise<SetupCompleteResult> {
		return parseSetupComplete(await this.call('setup.complete', {}, token));
	}

	async dispatch(
		token: string,
		module: string,
		action: string,
		payload: Record<string, unknown> = {}
	): Promise<Record<string, unknown>> {
		return this.call('module.dispatch', { module, action, payload }, token);
	}
}

let client: TetraClient | undefined;

/**
 * The process-wide Tetra client. Fixture mode (development and tests) uses an
 * in-memory mock that accepts `FIXTURE_SETUP_CODE`.
 */
export function tetraClient(): TetraClient {
	client ??=
		env.OOBE_FIXTURE_MODE === 'true'
			? new MockTetraClient()
			: new SocketTetraClient(env.TETRA_SOCKET || DEFAULT_TETRA_SOCKET);
	return client;
}

export interface TetraResponse {
	ok: boolean;
	payload?: Record<string, unknown>;
	error?: string;
}

/** Run a module action with the caller's setup session token. */
export async function dispatchTetra(
	token: string,
	module: string,
	action: string,
	payload: Record<string, unknown> = {}
): Promise<TetraResponse> {
	try {
		return { ok: true, payload: await tetraClient().dispatch(token, module, action, payload) };
	} catch (e) {
		if (e instanceof TetraError) return { ok: false, error: e.message };
		throw e;
	}
}

export async function tetraNetworkInterfaces(token: string): Promise<TetraResponse> {
	return dispatchTetra(token, 'network', 'interfaces');
}

export async function tetraNetworkScanWifi(
	token: string,
	interfaceName: string
): Promise<TetraResponse> {
	return dispatchTetra(token, 'network', 'scan_wifi', { interface: interfaceName, rescan: true });
}

/** `network.connect_wifi`: Tetra writes the profile and applies it safely, like `configure`. */
export async function tetraNetworkConnectWifi(
	token: string,
	interfaceName: string,
	ssid: string,
	hidden: boolean,
	security: WifiSecurityProfile
): Promise<TetraResponse> {
	return dispatchTetra(token, 'network', 'connect_wifi', {
		interface: interfaceName,
		ssid,
		hidden,
		security
	});
}

/**
 * `network.configure`. The result may hold `confirmation_required` and a
 * `confirmation_token` for `network.confirm`; see network-change.ts.
 */
export async function tetraNetworkConfigure(
	token: string,
	request: NetworkConfigureRequest
): Promise<TetraResponse> {
	return dispatchTetra(token, 'network', 'configure', { ...request });
}

/** `network.confirm`: keep a change Tetra would otherwise roll back. */
export async function tetraNetworkConfirm(
	token: string,
	confirmationToken: string
): Promise<TetraResponse> {
	return dispatchTetra(token, 'network', 'confirm', { token: confirmationToken });
}

export async function tetraSetHostname(token: string, hostname: string): Promise<TetraResponse> {
	return dispatchTetra(token, 'settings', 'set_hostname', { hostname });
}

export async function tetraSetKeymap(
	token: string,
	layout: string,
	variant?: string
): Promise<TetraResponse> {
	return dispatchTetra(token, 'settings', 'set_keymap', variant ? { layout, variant } : { layout });
}

export async function tetraCreateUser(token: string, name: string): Promise<TetraResponse> {
	return dispatchTetra(token, 'users', 'create', { name });
}

/** Make `name` an administrator (Tetra decides how, e.g. ultramarine-admin membership). */
export async function tetraAddAdmin(token: string, name: string): Promise<TetraResponse> {
	return dispatchTetra(token, 'users', 'add_admin', { name });
}

export async function tetraSetPassword(
	token: string,
	name: string,
	passwordHash: string
): Promise<TetraResponse> {
	return dispatchTetra(token, 'users', 'set_password', { name, password_hash: passwordHash });
}

export async function tetraSetAuthorizedKeys(
	token: string,
	name: string,
	keys: string[]
): Promise<TetraResponse> {
	return dispatchTetra(token, 'users', 'set_authorized_keys', { name, keys });
}

export async function tetraPower(
	token: string,
	action: 'reboot' | 'poweroff'
): Promise<TetraResponse> {
	return dispatchTetra(token, 'power', action);
}

export async function tetraSetUpdatePolicy(
	token: string,
	request: UpdatePolicyRequest
): Promise<TetraResponse> {
	return dispatchTetra(token, 'updates', 'set_policy', { ...request });
}

export async function tetraTailscaleUp(
	token: string,
	request: TailscaleUpRequest
): Promise<TetraResponse> {
	return dispatchTetra(token, 'remote_access', 'tailscale_up', { ...request });
}

export async function tetraCloudflaredSetToken(
	token: string,
	request: CloudflaredTokenRequest
): Promise<TetraResponse> {
	return dispatchTetra(token, 'remote_access', 'cloudflared_set_token', { ...request });
}

export async function tetraRemoteAccessStatus(
	token: string
): Promise<{ ok: true; status: RemoteAccessStatus } | { ok: false; error?: string }> {
	const resp = await dispatchTetra(token, 'remote_access', 'status');
	return resp.ok
		? { ok: true, status: parseRemoteAccessStatus(resp.payload ?? {}) }
		: { ok: false, error: resp.error };
}
