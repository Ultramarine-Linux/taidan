import { json, error } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { isIP } from 'node:net';
import { spawn } from 'node:child_process';
import { loadState, saveState } from '$lib/server/state';
import { SETUP_SESSION_COOKIE } from '$lib/server/setup-session';
import {
	tetraClient,
	TetraError,
	tetraAddAdmin,
	tetraCloudflaredSetToken,
	tetraCreateUser,
	tetraNetworkConnectWifi,
	tetraNetworkInterfaces,
	tetraNetworkScanWifi,
	tetraNetworkSetConfig,
	tetraNetworkStatus,
	tetraPower,
	tetraRemoteAccessStatus,
	tetraSetAuthorizedKeys,
	tetraSetHostname,
	tetraSetKeymap,
	tetraSetPassword,
	tetraSetUpdatePolicy,
	tetraTailscaleUp,
	type AccessRequest,
	type TetraResponse
} from '$lib/server/tetra';
import { validateDnsCredentials } from '$lib/dns-providers';
import type { StepId, OperationResult } from '$lib/oobe-state';

// Every operation that changes the host is a Tetra call made with the
// caller's setup session token. The OOBE runs unprivileged and changes
// nothing on the host itself.
const ALLOWED_OPERATIONS = new Set([
	'state.patch',
	'state.reset',
	'hostname.apply',
	'keyboard.apply',
	'user.create',
	'password.set',
	'network.interfaces',
	'network.scan_wifi',
	'network.connect_wifi',
	'network.set_config',
	'network.check',
	'ssh-key.apply',
	'updates.set_policy',
	'remote_access.status',
	'remote_access.tailscale_up',
	'remote_access.cloudflared_set_token',
	'access.set',
	'system.reboot',
	'system.poweroff',
	'setup.complete'
]);

const FIXTURE_MODE = process.env.OOBE_FIXTURE_MODE === 'true';

const HOSTNAME_PATTERN =
	/^[a-z0-9]([a-z0-9\-]{0,61}[a-z0-9])?(\.[a-z0-9]([a-z0-9\-]{0,61}[a-z0-9])?)*$/;
const DOMAIN_PATTERN = /^(?=.{1,253}$)([a-z0-9]([a-z0-9-]{0,61}[a-z0-9])?\.)+[a-z]{2,63}$/;
const SSH_KEY_PATTERN = /^(ssh-(rsa|ed25519|dss)|ecdsa-sha2-nistp\d+|sk-ssh-)[^\s]+\s+[^\s]+/;

// Every operation is validated by libtaidan (`tdnx server-oobe`) before it
// runs. There is no opt-out: if tdnx is missing or rejects the request, the
// operation fails. Fixture mode (development and tests only) has no tdnx.
function validateWithLibtaidan(
	step: StepId,
	operation: string,
	payload: Record<string, unknown>
): Promise<void> {
	if (FIXTURE_MODE) return Promise.resolve();
	return new Promise((resolve, reject) => {
		const child = spawn(process.env.TDNX_PATH || 'tdnx', ['server-oobe'], {
			stdio: ['pipe', 'pipe', 'pipe']
		});
		let stdout = '';
		let stderr = '';
		child.stdout.on('data', (data) => (stdout += data.toString()));
		child.stderr.on('data', (data) => (stderr += data.toString()));
		child.on('error', (e) => reject(new Error(`libtaidan validation unavailable: ${e.message}`)));
		child.on('close', (code) => {
			let response: { ok?: boolean; error?: string } | undefined;
			try {
				response = JSON.parse(stdout.trim()) as { ok?: boolean; error?: string };
			} catch {
				response = undefined;
			}
			if (code === 0 && response?.ok === true) {
				resolve();
				return;
			}
			reject(
				new Error(
					response?.error ||
						stderr.trim() ||
						(code === 0
							? 'Invalid response from libtaidan backend'
							: `libtaidan exited with code ${code}`)
				)
			);
		});
		child.stdin.end(JSON.stringify({ step, operation, payload }));
	});
}

function hashPassword(password: string): Promise<string> {
	return new Promise((resolve, reject) => {
		// The password is sent only on stdin. Tetra receives the resulting crypt
		// hash through its typed users.set_password operation, never plaintext.
		const child = spawn('openssl', ['passwd', '-6', '-stdin'], {
			stdio: ['pipe', 'pipe', 'pipe']
		});
		let stdout = '';
		let stderr = '';
		child.stdout.on('data', (data) => (stdout += data.toString()));
		child.stderr.on('data', (data) => (stderr += data.toString()));
		child.on('error', reject);
		child.on('close', (code) => {
			if (code !== 0) reject(new Error(stderr || `password hashing exited ${code}`));
			else resolve(stdout.trim());
		});
		child.stdin.end(`${password}\n`);
	});
}

interface OperationRequest {
	step: StepId;
	operation: string;
	payload?: Record<string, unknown>;
}

function result(
	id: string,
	step: StepId,
	status: OperationResult['status'],
	retryable: boolean,
	message?: string
): OperationResult {
	return { id, step, status, retryable, message };
}

function validateWifiSecurity(security: unknown): string | undefined {
	if (!security || typeof security !== 'object') return 'Wi-Fi security profile is required';
	const profile = security as Record<string, unknown>;
	const type = profile.type;
	if (!['open', 'wpa-personal', 'wpa-enterprise'].includes(type as string))
		return 'Unsupported Wi-Fi security type';
	if (type === 'wpa-personal' && (typeof profile.password !== 'string' || !profile.password)) {
		return 'Wi-Fi password is required';
	}
	if (type === 'wpa-enterprise') {
		if (typeof profile.eap !== 'string' || !profile.eap) return 'EAP method is required';
		if (typeof profile.username !== 'string' || !profile.username.trim())
			return 'Enterprise username is required';
		if (typeof profile.password !== 'string' || !profile.password)
			return 'Enterprise password is required';
		if (profile.phase2 !== undefined && typeof profile.phase2 !== 'string')
			return 'Invalid phase 2 method';
	}
	return undefined;
}

function validateNetworkConfig(payload: Record<string, unknown>): string | undefined {
	const iface = payload.interface;
	if (typeof iface !== 'string' || !/^[a-zA-Z0-9_.-]+$/.test(iface))
		return 'Invalid network interface';

	const ipv4 = payload.ipv4;
	if (!ipv4 || typeof ipv4 !== 'object') return 'IPv4 configuration is required';
	const config = ipv4 as Record<string, unknown>;
	if (config.method === 'dhcp') {
		// DHCP has no additional required values.
	} else if (config.method === 'static') {
		if (typeof config.address !== 'string' || isIP(config.address) !== 4)
			return 'Invalid static IPv4 address';
		if (
			typeof config.prefix_length !== 'number' ||
			config.prefix_length < 1 ||
			config.prefix_length > 32
		) {
			return 'Invalid IPv4 prefix length';
		}
		if (typeof config.gateway !== 'string' || isIP(config.gateway) !== 4)
			return 'Invalid IPv4 gateway';
		if (
			!Array.isArray(config.dns) ||
			config.dns.length === 0 ||
			config.dns.some((server) => typeof server !== 'string' || isIP(server) === 0)
		) {
			return 'At least one valid DNS server is required';
		}
	} else {
		return 'IPv4 method must be dhcp or static';
	}

	const wired = payload.wired8021x;
	if (wired !== undefined) {
		if (!wired || typeof wired !== 'object') return 'Invalid wired 802.1X configuration';
		const settings = wired as Record<string, unknown>;
		if (settings.enabled) {
			if (typeof settings.eap !== 'string' || !settings.eap) return 'Wired EAP method is required';
			if (typeof settings.username !== 'string' || !settings.username.trim())
				return 'Wired username is required';
			if (typeof settings.password !== 'string' || !settings.password)
				return 'Wired password is required';
		}
	}
}

/** Validate an access.set payload into Tetra's request. Returns an error message on failure. */
function accessRequest(payload: Record<string, unknown>): AccessRequest | string {
	const hostname = typeof payload.hostname === 'string' ? payload.hostname.trim() : '';
	if (payload.mode === 'ip') {
		const address = hostname.replace(/^\[|\]$/g, '');
		if (isIP(address) === 0) return 'Enter an IP address of this server';
		return { mode: 'ip', hostname: address };
	}
	if (payload.mode !== 'domain') return 'Access mode must be domain or ip';
	const domain = hostname.toLowerCase().replace(/\.$/, '');
	if (!DOMAIN_PATTERN.test(domain)) return 'Enter a fully qualified domain name';
	if (payload.challenge === 'http-01')
		return { mode: 'domain', hostname: domain, challenge: 'http-01' };
	if (payload.challenge !== 'dns-01') return 'Certificate challenge must be http-01 or dns-01';
	const provider = typeof payload.dns_provider === 'string' ? payload.dns_provider : '';
	const credentialError = validateDnsCredentials(provider, payload.dns_credentials);
	if (credentialError) return credentialError;
	const credentials = Object.fromEntries(
		Object.entries(payload.dns_credentials as Record<string, string>)
			.map(([key, value]) => [key, value.trim()])
			.filter(([, value]) => value)
	);
	return {
		mode: 'domain',
		hostname: domain,
		challenge: 'dns-01',
		dns_provider: provider,
		dns_credentials: credentials
	};
}

/** A single-line secret (auth key, tunnel token) with no whitespace. */
function secretValue(value: unknown, maxLength = 4096): string | undefined {
	if (typeof value !== 'string') return undefined;
	const trimmed = value.trim();
	return trimmed && trimmed.length <= maxLength && !/\s/.test(trimmed) ? trimmed : undefined;
}

export const POST: RequestHandler = async ({ request, locals, cookies }) => {
	// hooks.server.ts rejects requests without a live Tetra setup session.
	const token = locals.setupToken;
	if (!token) throw error(401, 'Setup session required');
	try {
		const body = (await request.json()) as OperationRequest;
		const { step, operation, payload = {} } = body;
		const opId = `op-${Date.now()}-${Math.random().toString(36).slice(2, 7)}`;
		if (!step || !ALLOWED_OPERATIONS.has(operation)) {
			return json(result(opId, step, 'failed', false, 'Unsupported setup operation'));
		}
		try {
			await validateWithLibtaidan(step, operation, payload);
		} catch (e) {
			return json(
				result(
					opId,
					step,
					'failed',
					true,
					e instanceof Error ? e.message : 'libtaidan validation failed'
				)
			);
		}

		/** Map a Tetra module response to an operation result. */
		const fromTetra = (resp: TetraResponse, failure: string, message?: string) =>
			json(
				resp.ok
					? result(opId, step, 'succeeded', false, message)
					: result(opId, step, 'failed', true, resp.error || failure)
			);

		const state = await loadState();

		switch (operation) {
			case 'state.patch': {
				// Only UI progress. Settings Tetra applies are recorded by their own
				// operations once Tetra accepts them.
				const patch = payload as Partial<typeof state>;
				if (patch.activeStep !== undefined) state.activeStep = patch.activeStep;
				if (patch.completed !== undefined) state.completed = patch.completed;
				if (patch.hostname !== undefined) state.hostname = patch.hostname;
				if (patch.administrator !== undefined) state.administrator = patch.administrator;
				if (patch.keyboardLayout !== undefined) state.keyboardLayout = patch.keyboardLayout;
				if (patch.keyboardVariant !== undefined) state.keyboardVariant = patch.keyboardVariant;
				if (patch.networkInterfaces !== undefined)
					state.networkInterfaces = patch.networkInterfaces;
				if (patch.steps !== undefined) {
					state.steps = state.steps.map((current) => {
						const update = patch.steps?.find((candidate) => candidate.id === current.id);
						return update ? { ...current, status: update.status } : current;
					});
				}
				await saveState(state);
				return json(result(opId, step, 'succeeded', false));
			}

			case 'state.reset': {
				// Development and tests only.
				if (!FIXTURE_MODE) return json(result(opId, step, 'failed', false, 'Not available'));
				state.activeStep = 'welcome';
				state.completed = false;
				state.steps = state.steps.map((current) => ({ ...current, status: 'pending' }));
				await saveState(state);
				return json(result(opId, step, 'succeeded', false));
			}

			case 'hostname.apply': {
				const hostname = payload.hostname as string;
				if (!hostname || !HOSTNAME_PATTERN.test(hostname)) {
					return json(result(opId, step, 'failed', true, 'Invalid hostname'));
				}
				const resp = await tetraSetHostname(token, hostname);
				if (resp.ok) {
					state.hostname = hostname;
					await saveState(state);
				}
				return fromTetra(resp, 'Failed to set hostname');
			}

			case 'keyboard.apply': {
				const layout = payload.layout as string;
				const variant = (payload.variant as string) || '';
				if (!layout || !/^[a-zA-Z0-9_-]+$/.test(layout)) {
					return json(result(opId, step, 'failed', true, 'Invalid keyboard layout'));
				}
				if (variant && !/^[a-zA-Z0-9_-]+$/.test(variant)) {
					return json(result(opId, step, 'failed', true, 'Invalid keyboard variant'));
				}
				const resp = await tetraSetKeymap(token, layout, variant || undefined);
				if (resp.ok) {
					state.keyboardLayout = layout;
					state.keyboardVariant = variant || undefined;
					await saveState(state);
				}
				return fromTetra(resp, 'Keyboard setup failed');
			}

			case 'user.create': {
				const name = payload.name as string;
				if (!name || !/^[a-z_][a-z0-9_-]*$/.test(name)) {
					return json(result(opId, step, 'failed', true, 'Invalid username'));
				}
				// Every host change goes through Tetra. If Tetra cannot create the
				// account, the step fails; there is no local fallback.
				const created = await tetraCreateUser(token, name);
				if (!created.ok) return fromTetra(created, 'Tetra could not create user');
				// setup.complete requires an eligible administrator.
				const admin = await tetraAddAdmin(token, name);
				if (!admin.ok) return fromTetra(admin, 'Tetra could not make the user an administrator');
				state.administrator = name;
				await saveState(state);
				return json(result(opId, step, 'succeeded', false));
			}

			case 'password.set': {
				const name = payload.name as string;
				const plaintext = payload.password as string;
				if (!name || !plaintext) {
					return json(result(opId, step, 'failed', true, 'Missing user or password'));
				}
				let passwordHash: string;
				try {
					passwordHash = await hashPassword(plaintext);
				} catch (e) {
					return json(
						result(opId, step, 'failed', true, e instanceof Error ? e.message : 'Hashing failed')
					);
				}
				return fromTetra(
					await tetraSetPassword(token, name, passwordHash),
					'Tetra could not set password'
				);
			}

			case 'ssh-key.apply': {
				const publicKey = typeof payload.publicKey === 'string' ? payload.publicKey.trim() : '';
				if (!SSH_KEY_PATTERN.test(publicKey) || publicKey.includes('\n')) {
					return json(result(opId, step, 'failed', true, 'Invalid SSH public key'));
				}
				if (!state.administrator) {
					return json(result(opId, step, 'failed', true, 'Create the administrator first'));
				}
				const resp = await tetraSetAuthorizedKeys(token, state.administrator, [publicKey]);
				if (resp.ok) {
					state.sshPublicKey = publicKey;
					await saveState(state);
				}
				return fromTetra(resp, 'Failed to install SSH key');
			}

			case 'network.interfaces': {
				const resp = await tetraNetworkInterfaces(token);
				if (!resp.ok) return fromTetra(resp, 'Network query failed');
				const interfaces = Array.isArray(resp.payload?.interfaces) ? resp.payload.interfaces : [];
				state.networkInterfaces = interfaces as typeof state.networkInterfaces;
				await saveState(state);
				return json(result(opId, step, 'succeeded', false, JSON.stringify(interfaces)));
			}

			case 'network.scan_wifi': {
				const interfaceName = payload.interface as string;
				if (!interfaceName || !/^[a-zA-Z0-9_.-]+$/.test(interfaceName)) {
					return json(result(opId, step, 'failed', true, 'Invalid Wi-Fi interface'));
				}
				const resp = await tetraNetworkScanWifi(token, interfaceName);
				if (!resp.ok) return fromTetra(resp, 'Wi-Fi scan failed');
				const networks = Array.isArray(resp.payload?.networks) ? resp.payload.networks : [];
				return json(result(opId, step, 'succeeded', false, JSON.stringify(networks)));
			}

			case 'network.connect_wifi': {
				const interfaceName = payload.interface as string;
				const ssid = payload.ssid as string;
				if (!interfaceName || !/^[a-zA-Z0-9_.-]+$/.test(interfaceName) || !ssid) {
					return json(result(opId, step, 'failed', true, 'Wi-Fi interface and SSID are required'));
				}
				const securityError = validateWifiSecurity(payload.security);
				if (securityError) return json(result(opId, step, 'failed', true, securityError));
				return fromTetra(
					await tetraNetworkConnectWifi(
						token,
						interfaceName,
						ssid,
						payload.security as Record<string, unknown>
					),
					'Wi-Fi connection failed'
				);
			}

			case 'network.set_config': {
				const validationError = validateNetworkConfig(payload);
				if (validationError) return json(result(opId, step, 'failed', true, validationError));
				return fromTetra(
					await tetraNetworkSetConfig(token, payload),
					'Network configuration failed'
				);
			}

			case 'network.check':
				return fromTetra(
					await tetraNetworkStatus(token),
					'Network check failed',
					'Network is available'
				);

			case 'updates.set_policy': {
				const policy = payload.policy;
				if (policy !== 'automatic' && policy !== 'notify') {
					return json(result(opId, step, 'failed', true, 'Choose an update policy'));
				}
				const resp = await tetraSetUpdatePolicy(token, { policy });
				if (resp.ok) {
					state.updatePolicy = policy;
					await saveState(state);
				}
				return fromTetra(resp, 'Failed to save the update policy');
			}

			case 'remote_access.status': {
				const resp = await tetraRemoteAccessStatus(token);
				if (!resp.ok) return fromTetra(resp, 'Failed to read remote access status');
				return json(result(opId, step, 'succeeded', false, JSON.stringify(resp.status)));
			}

			case 'remote_access.tailscale_up': {
				// The auth key goes straight to Tetra and is never stored.
				const authKey = secretValue(payload.auth_key, 512);
				if (!authKey || !authKey.startsWith('tskey-')) {
					return json(result(opId, step, 'failed', true, 'Enter a Tailscale auth key (tskey-…)'));
				}
				const serve = payload.serve !== false;
				const funnel = payload.funnel === true;
				// Funnel publishes the dashboard on the internet; the user must
				// have confirmed the warning.
				if (funnel && payload.confirmFunnel !== true) {
					return json(
						result(opId, step, 'failed', true, 'Confirm that the dashboard will be public')
					);
				}
				if (funnel && !serve) {
					return json(result(opId, step, 'failed', true, 'Funnel requires Tailscale Serve'));
				}
				const resp = await tetraTailscaleUp(token, { auth_key: authKey, serve, funnel });
				if (resp.ok) {
					state.remoteAccess = {
						...state.remoteAccess,
						tailscale: true,
						tailscaleServe: serve,
						tailscaleFunnel: funnel
					};
					await saveState(state);
				}
				return fromTetra(resp, 'Failed to connect Tailscale');
			}

			case 'remote_access.cloudflared_set_token': {
				// The tunnel token goes straight to Tetra and is never stored.
				const tunnelToken = secretValue(payload.token);
				if (!tunnelToken) {
					return json(result(opId, step, 'failed', true, 'Enter a Cloudflare Tunnel token'));
				}
				const resp = await tetraCloudflaredSetToken(token, { token: tunnelToken });
				if (resp.ok) {
					state.remoteAccess = { ...state.remoteAccess, cloudflared: true };
					await saveState(state);
				}
				return fromTetra(resp, 'Failed to configure Cloudflare Tunnel');
			}

			case 'access.set': {
				const access = accessRequest(payload);
				if (typeof access === 'string') return json(result(opId, step, 'failed', true, access));
				try {
					const applied = await tetraClient().setAccess(token, access);
					// DNS provider credentials are deliberately left out.
					state.access = {
						mode: access.mode,
						hostname: access.hostname,
						challenge: access.mode === 'domain' ? access.challenge : 'http-01',
						dnsProvider:
							access.mode === 'domain' && access.challenge === 'dns-01'
								? access.dns_provider
								: undefined,
						applied: true,
						passkeysAvailable: applied.passkeysAvailable
					};
					await saveState(state);
					return json(result(opId, step, 'succeeded', false, JSON.stringify(applied)));
				} catch (e) {
					if (!(e instanceof TetraError)) throw e;
					return json(result(opId, step, 'failed', true, e.message));
				}
			}

			case 'system.reboot':
				return fromTetra(await tetraPower(token, 'reboot'), 'Failed to reboot', 'Rebooting…');

			case 'system.poweroff':
				return fromTetra(
					await tetraPower(token, 'poweroff'),
					'Failed to shut down',
					'Shutting down…'
				);

			case 'setup.complete': {
				// Tetra disables the setup code, ends every setup session (this one
				// included), starts the dashboard and stops the OOBE shortly after.
				// It refuses while no eligible administrator exists.
				try {
					const completed = await tetraClient().completeSetup(token);
					state.completed = true;
					await saveState(state);
					cookies.delete(SETUP_SESSION_COOKIE, { path: '/' });
					return json(result(opId, step, 'succeeded', false, JSON.stringify(completed)));
				} catch (e) {
					if (!(e instanceof TetraError)) throw e;
					const message =
						e.code === 'setup_incomplete'
							? 'Create an administrator account before finishing setup'
							: e.message;
					return json(result(opId, step, 'failed', e.code !== 'setup_complete', message));
				}
			}

			default:
				return json(result(opId, step, 'failed', false, `Unknown operation: ${operation}`));
		}
	} catch (err) {
		console.error('Operation failed:', err instanceof Error ? err.message : 'unknown error');
		throw error(500, 'Internal server error');
	}
};
