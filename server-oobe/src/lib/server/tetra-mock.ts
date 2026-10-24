// In-memory stand-in for Tetra's local session API, used in fixture mode and
// tests until Tetra's setup API is merged and running on development machines.
// It follows docs/auth-protocol.md and the agreed setup actions closely enough
// to exercise the OOBE: wrong codes are rejected, tokens are checked on every
// call, the setup session may only run SETUP_ALLOWED actions (so
// `network.set_config` is refused), and `setup.complete` ends every setup
// session.
//
// Each setup session sees its own fixture host (fixture-hosts.ts), so tests
// running in parallel do not share network state. Tetra holds one network
// change awaiting confirmation per host; the mock holds one per session.
//
// This module must not import SvelteKit aliases ($lib, $app, $env) so that
// tests can load it directly.

import { randomBytes } from 'node:crypto';
import { isIP } from 'node:net';
import { FIXTURE_SETUP_CODE } from '../fixture';
import { fixtureHost, fixtureInterface, type FixtureHost } from '../fixture-hosts';
import { isRoutable } from '../net';
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
/** Tetra's `ROLLBACK_AFTER`. */
export const ROLLBACK_SECONDS = 60;

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

interface MockSession {
	expiresAt: number;
	host: FixtureHost;
	/** The change awaiting `network.confirm`, if any. */
	pending?: { token: string; interface: string; previous: FixtureHost['interfaces'][number] };
}

function badRequest(message: string): TetraError {
	return new TetraError('bad_request', message);
}

/** Tetra deserializes with `deny_unknown_fields`. */
function denyUnknown(value: unknown, fields: string[], what: string): Record<string, unknown> {
	if (!value || typeof value !== 'object' || Array.isArray(value)) {
		throw badRequest(`invalid ${what}`);
	}
	for (const key of Object.keys(value)) {
		if (!fields.includes(key)) throw badRequest(`unknown field \`${key}\` in ${what}`);
	}
	return value as Record<string, unknown>;
}

const IP_SECTION_FIELDS = ['method', 'addresses', 'gateway', 'dns'];
/** Tetra's `Wired8021x`, also the `wpa-enterprise` fields. */
const EAP_FIELDS = [
	'eap',
	'identity',
	'anonymous_identity',
	'password',
	'ca_cert',
	'client_cert',
	'private_key',
	'private_key_password',
	'domain_suffix_match',
	'phase2'
];

/** The addresses an `ipv4`/`ipv6` section leaves, as CIDRs; `null` keeps the current ones. */
function sectionAddresses(section: unknown, family: 4 | 6): string[] | null {
	const config = denyUnknown(section ?? {}, IP_SECTION_FIELDS, `ipv${family}`);
	const method = config.method ?? 'auto';
	const methods =
		family === 4
			? ['auto', 'manual', 'disabled']
			: ['auto', 'dhcp', 'manual', 'link-local', 'disabled'];
	if (!methods.includes(method as string)) throw badRequest(`unknown ipv${family} method`);
	const addresses = (config.addresses ?? []) as unknown[];
	const sameFamily = (value: unknown) => typeof value === 'string' && isIP(value) === family;
	if ((method === 'manual') !== addresses.length > 0) {
		throw badRequest(`ipv${family}.addresses are required with, and only allowed with, manual`);
	}
	if (config.gateway !== undefined && (method !== 'manual' || !sameFamily(config.gateway))) {
		throw badRequest(`invalid ipv${family}.gateway`);
	}
	if (!((config.dns ?? []) as unknown[]).every(sameFamily))
		throw badRequest(`invalid ipv${family}.dns`);
	if (method === 'manual') {
		return addresses.map((entry) => {
			const { address, prefix_length } = denyUnknown(
				entry,
				['address', 'prefix_length'],
				'address'
			);
			if (!sameFamily(address)) throw badRequest(`invalid ipv${family} address`);
			return `${address}/${prefix_length}`;
		});
	}
	return method === 'disabled' || method === 'link-local' ? [] : null;
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
	private readonly sessions = new Map<string, MockSession>();
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

	private requireSession(token: string): MockSession {
		const session = this.sessions.get(token);
		if (session === undefined || session.expiresAt <= this.now()) {
			this.sessions.delete(token);
			throw new TetraError('unauthenticated', 'a valid session token is required; log in again');
		}
		return session;
	}

	private dashboardUrl(): string | undefined {
		if (!this.access) return undefined;
		const host = this.access.hostname;
		return `https://${host.includes(':') ? `[${host}]` : host}`;
	}

	/** Fixture mode: make the session `token` see the fixture host `name`. */
	useFixtureHost(token: string, name: string | undefined) {
		this.requireSession(token).host = fixtureHost(name);
	}

	/** The interfaces the session `token` sees. */
	interfaces(token: string): FixtureHost['interfaces'] {
		return this.requireSession(token).host.interfaces;
	}

	/**
	 * Tetra's safe apply: replace `name`'s addresses, and when none of its
	 * routable addresses survive, hold the change for `network.confirm` and
	 * roll it back after ROLLBACK_SECONDS.
	 */
	private applyAddresses(session: MockSession, name: string, cidrs: string[], operstate = 'up') {
		if (session.pending) {
			throw new TetraError(
				'action_failed',
				'another network change is awaiting confirmation; confirm it or wait for it to roll back'
			);
		}
		const index = session.host.interfaces.findIndex((entry) => entry.name === name);
		if (index === -1) throw new TetraError('action_failed', `no such interface: ${name}`);
		const previous = session.host.interfaces[index];
		const next = fixtureInterface(name, previous.type, operstate, previous.mac, cidrs);
		session.host.interfaces[index] = next;
		const routable = (entry: typeof next) =>
			entry.addresses.filter((address) => isRoutable(address.scope)).map((a) => a.address);
		const before = routable(previous);
		const after = routable(next);
		const response = {
			path: `/etc/NetworkManager/system-connections/${name}.nmconnection`,
			applied: true,
			activation_error: null,
			addresses_before: before
		};
		if (before.length === 0 || before.some((address) => after.includes(address))) {
			return { ...response, confirmation_required: false, addresses_after: after };
		}
		const token = randomBytes(16).toString('hex');
		session.pending = { token, interface: name, previous };
		setTimeout(() => {
			if (session.pending?.token !== token) return;
			session.pending = undefined;
			const at = session.host.interfaces.findIndex((entry) => entry.name === name);
			if (at !== -1) session.host.interfaces[at] = previous;
		}, ROLLBACK_SECONDS * 1000).unref?.();
		return {
			...response,
			confirmation_required: true,
			confirmation_token: token,
			rollback_in_seconds: ROLLBACK_SECONDS,
			addresses_after: []
		};
	}

	private configure(session: MockSession, payload: Record<string, unknown>) {
		const request = denyUnknown(
			payload,
			['interface', 'name', 'ipv4', 'ipv6', 'wired_8021x', 'dry_run'],
			'network.configure'
		);
		const name = String(request.interface ?? '');
		const current = session.host.interfaces.find((entry) => entry.name === name);
		if (!current) throw new TetraError('action_failed', `no such interface: ${name}`);
		if (current.type !== 'ethernet') {
			throw new TetraError('action_failed', `${name} is not an Ethernet interface`);
		}
		if (request.wired_8021x !== undefined)
			denyUnknown(request.wired_8021x, EAP_FIELDS, 'wired_8021x');
		const ipv4 = sectionAddresses(request.ipv4, 4);
		const ipv6 = sectionAddresses(request.ipv6, 6);
		// `null` (auto, dhcp) counts as routable, as in Tetra.
		if (ipv4?.length === 0 && ipv6?.length === 0) {
			throw badRequest('at least one address family must give the host a routable address');
		}
		const cidrs = (family: 'ipv4' | 'ipv6', scope?: string) =>
			current.addresses
				.filter((address) => address.family === family && (!scope || address.scope === scope))
				.map((address) => `${address.address}/${address.prefix_length}`);
		const ipv6Method = (request.ipv6 as Record<string, unknown> | undefined)?.method;
		const v6 =
			ipv6 === null
				? cidrs('ipv6')
				: ipv6Method === 'disabled'
					? []
					: [...ipv6, ...cidrs('ipv6', 'link-local')];
		return this.applyAddresses(session, name, [...(ipv4 ?? cidrs('ipv4')), ...v6]);
	}

	private connectWifi(session: MockSession, payload: Record<string, unknown>) {
		const request = denyUnknown(
			payload,
			['interface', 'ssid', 'hidden', 'security', 'dry_run'],
			'network.connect_wifi'
		);
		const security = request.security as Record<string, unknown> | undefined;
		const fields: Record<string, string[]> = {
			open: ['type'],
			'wpa-personal': ['type', 'password'],
			'wpa-enterprise': ['type', ...EAP_FIELDS]
		};
		const allowed = fields[String(security?.type)];
		if (!allowed) throw badRequest('unknown security type');
		denyUnknown(security, allowed, 'security');
		const name = String(request.interface ?? '');
		const current = session.host.interfaces.find((entry) => entry.name === name);
		if (current?.type !== 'wifi')
			throw new TetraError('action_failed', `${name} is not a wireless interface`);
		for (const network of session.host.wifiNetworks) network.in_use = network.ssid === request.ssid;
		// The fixture Wi-Fi hands out the same addresses again, or these on first connect.
		const cidrs = current.addresses.length
			? current.addresses.map((address) => `${address.address}/${address.prefix_length}`)
			: ['192.168.50.21/24', '2001:db8:50::21/64', 'fe80::5054:ff:feab:cdef/64'];
		return this.applyAddresses(session, name, cidrs);
	}

	private confirm(payload: Record<string, unknown>) {
		// Tetra's pending change is per host, not per session: any session may confirm it.
		for (const session of this.sessions.values()) {
			if (session.pending && session.pending.token === payload.token) {
				const { interface: name } = session.pending;
				session.pending = undefined;
				return {
					confirmed: true,
					path: `/etc/NetworkManager/system-connections/${name}.nmconnection`
				};
			}
		}
		throw new TetraError(
			'action_failed',
			'no network change is awaiting confirmation with that token'
		);
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
		this.sessions.set(token, { expiresAt, host: fixtureHost() });
		return { token, expiresAt, adminExists: this.adminExists };
	}

	async sessionInfo(token: string): Promise<SessionInfo> {
		const { expiresAt } = this.requireSession(token);
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
		const session = this.requireSession(token);
		if (!SETUP_ALLOWED.has(`${module}.${action}`) && action !== 'capabilities') {
			throw new TetraError('forbidden', `the setup session may not run ${module}.${action}`);
		}
		this.calls.push({ module, action, payload });
		switch (`${module}.${action}`) {
			case 'users.add_admin':
				this.adminExists = true;
				return { name: payload.name };
			case 'network.interfaces':
				return { interfaces: structuredClone(session.host.interfaces) };
			case 'network.status': {
				const name = payload.interface;
				return {
					addresses: session.host.interfaces
						.filter((entry) => name === undefined || entry.name === name)
						.flatMap((entry) => entry.addresses)
				};
			}
			case 'network.scan_wifi': {
				const request = denyUnknown(payload, ['interface', 'rescan'], 'network.scan_wifi');
				const name = String(request.interface ?? '');
				if (
					!session.host.interfaces.some((entry) => entry.name === name && entry.type === 'wifi')
				) {
					throw new TetraError('action_failed', `${name} is not a wireless interface`);
				}
				return { networks: structuredClone(session.host.wifiNetworks) };
			}
			case 'network.configure':
				return this.configure(session, payload);
			case 'network.connect_wifi':
				return this.connectWifi(session, payload);
			case 'network.confirm':
				return this.confirm(payload);
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
