// In-memory stand-in for Tetra's local session API, used in fixture mode and
// tests until Tetra's setup API is merged and running on development machines.
// It follows docs/auth-protocol.md and the agreed setup actions closely enough
// to exercise the OOBE: wrong codes are rejected, tokens are checked on every
// call, the setup session may only run SETUP_ALLOWED actions, and
// `setup.complete` ends every setup session.
//
// This module must not import SvelteKit aliases ($lib, $app, $env) so that
// tests can load it directly.

import { randomBytes } from 'node:crypto';
import { FIXTURE_SETUP_CODE } from '../fixture';
import {
	TetraError,
	type AccessRequest,
	type AccessResult,
	type RemoteAccessStatus,
	type SessionInfo,
	type SetupCompleteResult,
	type SetupSession,
	type SetupStatus,
	type TetraClient
} from './tetra-api';

const SETUP_SESSION_SECONDS = 2 * 60 * 60;
const HANDOFF_SECONDS = 5;

/** The module actions the setup session may run (Tetra's SETUP_ALLOWED). */
export const SETUP_ALLOWED = new Set([
	'agent.capabilities',
	'settings.set_hostname',
	'settings.set_keymap',
	'settings.set_locale',
	'settings.set_timezone',
	'users.create',
	'users.set_password',
	'users.set_authorized_keys',
	'users.add_admin',
	'power.reboot',
	'power.poweroff',
	'network.interfaces',
	'network.status',
	'network.scan_wifi',
	'network.connect_wifi',
	'network.configure',
	'network.confirm',
	'updates.set_policy',
	'remote_access.tailscale_up',
	'remote_access.cloudflared_set_token',
	'remote_access.status'
]);

export interface MockRedeemAttempt {
	remoteAddr?: string;
	ok: boolean;
}

export interface MockDispatch {
	module: string;
	action: string;
	payload: Record<string, unknown>;
}

function normalize(code: string): string {
	return code.toUpperCase().replace(/[^0-9A-Z]/g, '');
}

export class MockTetraClient implements TetraClient {
	/** Every setup.redeem attempt and the address it came from, oldest first. */
	readonly redeemAttempts: MockRedeemAttempt[] = [];
	/** Every module action dispatched with a valid token, oldest first. */
	readonly calls: MockDispatch[] = [];
	/** The last accepted access.set request, credentials included. */
	access: AccessRequest | undefined;
	readonly remoteAccess: RemoteAccessStatus = {
		tailscale: { enabled: false, serve: false, funnel: false },
		cloudflared: { enabled: false }
	};
	private readonly sessions = new Map<string, number>();
	private complete = false;
	private adminExists: boolean;

	constructor(
		private readonly code: string = FIXTURE_SETUP_CODE,
		options: { adminExists?: boolean } = {}
	) {
		this.adminExists = options.adminExists ?? false;
	}

	private now() {
		return Math.floor(Date.now() / 1000);
	}

	private requireSession(token: string): number {
		const expiresAt = this.sessions.get(token);
		if (expiresAt === undefined || expiresAt <= this.now()) {
			this.sessions.delete(token);
			throw new TetraError('unauthenticated', 'a valid session token is required; log in again');
		}
		return expiresAt;
	}

	private dashboardUrl(): string | undefined {
		return this.access ? `https://${this.access.hostname}` : undefined;
	}

	async setupStatus(): Promise<SetupStatus> {
		return { setupRequired: !this.complete, passkeysAvailable: this.access?.mode === 'domain' };
	}

	async redeemSetupCode(code: string, remoteAddr?: string): Promise<SetupSession> {
		if (this.complete) throw new TetraError('setup_complete', 'setup has already completed');
		const ok = normalize(code) === normalize(this.code);
		this.redeemAttempts.push({ remoteAddr, ok });
		if (!ok) throw new TetraError('invalid_credentials', 'invalid credentials');
		const token = `tts_${randomBytes(32).toString('base64url')}`;
		const expiresAt = this.now() + SETUP_SESSION_SECONDS;
		this.sessions.set(token, expiresAt);
		return { token, expiresAt, adminExists: this.adminExists };
	}

	async sessionInfo(token: string): Promise<SessionInfo> {
		const expiresAt = this.requireSession(token);
		return { kind: 'setup', client: 'oobe', expiresAt };
	}

	async setAccess(token: string, request: AccessRequest): Promise<AccessResult> {
		this.requireSession(token);
		if (request.mode === 'domain' && request.challenge === 'dns-01') {
			if (!request.dns_provider || Object.keys(request.dns_credentials ?? {}).length === 0) {
				throw new TetraError('bad_request', 'dns-01 needs a provider and credentials');
			}
		}
		this.access = request;
		return { passkeysAvailable: request.mode === 'domain', dashboardUrl: this.dashboardUrl() };
	}

	async completeSetup(token: string): Promise<SetupCompleteResult> {
		this.requireSession(token);
		if (!this.adminExists) {
			throw new TetraError('setup_incomplete', 'no eligible administrator exists');
		}
		this.complete = true;
		this.sessions.clear();
		return { dashboardUrl: this.dashboardUrl(), handoffInSeconds: HANDOFF_SECONDS };
	}

	async dispatch(
		token: string,
		module: string,
		action: string,
		payload: Record<string, unknown> = {}
	): Promise<Record<string, unknown>> {
		this.requireSession(token);
		if (!SETUP_ALLOWED.has(`${module}.${action}`) && action !== 'capabilities') {
			throw new TetraError('forbidden', `the setup session may not run ${module}.${action}`);
		}
		this.calls.push({ module, action, payload });
		switch (`${module}.${action}`) {
			case 'users.add_admin':
				this.adminExists = true;
				return { name: payload.name };
			case 'network.interfaces':
				return { interfaces: [] };
			case 'network.scan_wifi':
				return { networks: [] };
			case 'updates.set_policy':
				return { policy: payload.policy };
			case 'remote_access.tailscale_up':
				this.remoteAccess.tailscale = {
					enabled: true,
					serve: payload.serve === true,
					funnel: payload.funnel === true,
					hostname: 'server.example-tailnet.ts.net'
				};
				return { ...this.remoteAccess };
			case 'remote_access.cloudflared_set_token':
				this.remoteAccess.cloudflared = { enabled: true };
				return { ...this.remoteAccess };
			case 'remote_access.status':
				return { ...this.remoteAccess };
			default:
				return {};
		}
	}
}
