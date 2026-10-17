import { json, error } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { loadState, saveState, DEFAULT_OOBE_PORT } from '$lib/server/state';
import { access, readFile, writeFile, mkdir } from 'node:fs/promises';
import { constants } from 'node:fs';
import { env } from 'node:process';
import { setTimeout as delay } from 'node:timers/promises';
import {
	tetraSetHostname,
	tetraCreateUser,
	tetraSetPassword,
	tetraNetworkStatus,
	tetraNetworkInterfaces,
	tetraNetworkSetConfig,
	tetraNetworkScanWifi,
	tetraNetworkConnectWifi,
	tetraServiceStatus,
	tetraServiceStart,
	tetraServiceEnable,
	tetraCapabilities
} from '$lib/server/tetra';
import { execFile, spawn } from 'node:child_process';
import { promisify } from 'node:util';
import { isIP } from 'node:net';
import type { StepId, OperationResult } from '$lib/oobe-state';

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
	'tetra.detect',
	'tetra.start',
	'ssh-key.apply',
	'tweaks.apply',
	'fyra.begin',
	'system.reboot',
	'system.poweroff',
	'dashboard.install',
	'dashboard.handoff',
	'cloudflare.install'
]);

const execFileAsync = promisify(execFile);

const LIBTAIDAN_ENABLED = process.env.OOBE_LIBTAIDAN === 'true';

function validateWithLibtaidan(
	step: StepId,
	operation: string,
	payload: Record<string, unknown>
): Promise<void> {
	if (!LIBTAIDAN_ENABLED) return Promise.resolve();
	return new Promise((resolve, reject) => {
		const child = spawn(process.env.TDNX_PATH || 'tdnx', ['server-oobe'], {
			stdio: ['pipe', 'pipe', 'pipe']
		});
		let stdout = '';
		let stderr = '';
		child.stdout.on('data', (data) => (stdout += data.toString()));
		child.stderr.on('data', (data) => (stderr += data.toString()));
		child.on('error', reject);
		child.on('close', (code) => {
			if (code !== 0) {
				reject(new Error(stderr.trim() || stdout.trim() || `libtaidan exited with code ${code}`));
				return;
			}
			try {
				const response = JSON.parse(stdout.trim()) as { ok?: boolean; error?: string };
				if (response.ok !== true)
					reject(new Error(response.error || 'libtaidan rejected operation'));
				else resolve();
			} catch {
				reject(new Error('Invalid response from libtaidan backend'));
			}
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

const FIXTURE_MODE = process.env.OOBE_FIXTURE_MODE === 'true';

export const POST: RequestHandler = async ({ request }) => {
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

		const state = await loadState();

		if (operation === 'state.patch') {
			const patch = payload as Partial<typeof state>;
			if (patch.activeStep !== undefined) state.activeStep = patch.activeStep;
			if (patch.completed !== undefined) state.completed = patch.completed;
			if (patch.hostname !== undefined) state.hostname = patch.hostname;
			if (patch.administrator !== undefined) state.administrator = patch.administrator;
			if (patch.keyboardLayout !== undefined) state.keyboardLayout = patch.keyboardLayout;
			if (patch.keyboardVariant !== undefined) state.keyboardVariant = patch.keyboardVariant;
			if (patch.tweaks !== undefined) state.tweaks = patch.tweaks;
			if (patch.networkInterfaces !== undefined) state.networkInterfaces = patch.networkInterfaces;

			if (patch.dashboardDomain !== undefined) state.dashboardDomain = patch.dashboardDomain;
			if (patch.dashboardPort !== undefined) state.dashboardPort = patch.dashboardPort;
			if (patch.steps !== undefined) {
				state.steps = state.steps.map((current) => {
					const update = patch.steps?.find((candidate) => candidate.id === current.id);
					return update ? { ...current, status: update.status } : current;
				});
			}
			await saveState(state);
			return json(result(opId, step, 'succeeded', false));
		}

		if (FIXTURE_MODE) {
			// Fixture mode: accept any operation, update state where applicable, return success.
			switch (operation) {
				case 'state.reset':
					state.activeStep = 'welcome';
					state.completed = false;
					state.steps = state.steps.map((step) => ({ ...step, status: 'pending' }));
					await saveState(state);
					return json(result(opId, step, 'succeeded', false));
				case 'hostname.apply':
					state.hostname = payload.hostname as string;
					await saveState(state);
					return json(result(opId, step, 'succeeded', false));
				case 'keyboard.apply':
					state.keyboardLayout = payload.layout as string;
					state.keyboardVariant = payload.variant as string | undefined;
					await saveState(state);
					return json(result(opId, step, 'succeeded', false));
				case 'user.create':
					state.administrator = payload.name as string;
					await saveState(state);
					return json(result(opId, step, 'succeeded', false));
				case 'password.set':
					return json(result(opId, step, 'succeeded', false));
				case 'network.check':
					return json(result(opId, step, 'succeeded', false, 'Network is available'));
				case 'network.interfaces':
					return json(result(opId, step, 'succeeded', false));
				case 'network.set_config':
					return json(result(opId, step, 'succeeded', false));
				case 'network.scan_wifi':
					return json(
						result(
							opId,
							step,
							'succeeded',
							false,
							JSON.stringify([
								{ ssid: 'Ultramarine-Setup', security: 'wpa2', signal: 86 },
								{ ssid: 'Guest Network', security: 'open', signal: 58 }
							])
						)
					);
				case 'network.connect_wifi':
					return json(
						result(opId, step, 'succeeded', false, 'Wi-Fi connection accepted in fixture mode')
					);
				case 'tetra.start':
					return json(result(opId, step, 'succeeded', false, 'Tetra is enabled by default'));
				case 'tweaks.apply':
					return json(result(opId, step, 'succeeded', false));
				case 'ssh-key.apply':
					return json(result(opId, step, 'succeeded', false));
				case 'dashboard.install':
					return json(result(opId, step, 'succeeded', false, 'Dashboard service installed'));
				case 'dashboard.handoff':
					return json(result(opId, step, 'succeeded', false, 'Handoff scheduled'));
				case 'cloudflare.install':
					state.cloudflare.installed = true;
					state.cloudflare.installing = false;
					await saveState(state);
					return json(result(opId, step, 'succeeded', false, 'Cloudflared installed'));
				case 'fyra.begin':
					state.fyra.status = 'pending';
					await saveState(state);
					return json(result(opId, step, 'succeeded', false, 'Fyra authorization started'));
				default:
					return json(result(opId, step, 'succeeded', false, `Fixture mode: ${operation}`));
			}
		}

		switch (operation) {
			case 'hostname.apply': {
				const hostname = payload.hostname as string;
				if (
					!hostname ||
					!/^[a-z0-9]([a-z0-9\-]{0,61}[a-z0-9])?(\.[a-z0-9]([a-z0-9\-]{0,61}[a-z0-9])?)*$/.test(
						hostname
					)
				) {
					return json(result(opId, step, 'failed', true, 'Invalid hostname'));
				}
				let resp: Awaited<ReturnType<typeof tetraSetHostname>>;
				try {
					resp = await tetraSetHostname(hostname);
				} catch (e: any) {
					return json(result(opId, step, 'failed', true, e.message || 'Failed to set hostname'));
				}
				if (!resp.ok) {
					return json(result(opId, step, 'failed', true, resp.error || 'Failed to set hostname'));
				}
				state.hostname = hostname;
				await saveState(state);
				return json(result(opId, step, 'succeeded', false));
			}

			case 'network.interfaces': {
				try {
					const resp = await tetraNetworkInterfaces();
					if (!resp.ok)
						return json(result(opId, step, 'failed', true, resp.error || 'Network query failed'));
					const interfaces = Array.isArray(resp.payload?.interfaces) ? resp.payload.interfaces : [];
					state.networkInterfaces = interfaces as typeof state.networkInterfaces;
					await saveState(state);
					return json(result(opId, step, 'succeeded', false, JSON.stringify(interfaces)));
				} catch (e: any) {
					return json(result(opId, step, 'failed', true, e.message || 'Network query failed'));
				}
			}

			case 'network.scan_wifi': {
				const interfaceName = payload.interface as string;
				if (!interfaceName || !/^[a-zA-Z0-9_.-]+$/.test(interfaceName)) {
					return json(result(opId, step, 'failed', true, 'Invalid Wi-Fi interface'));
				}
				try {
					const resp = await tetraNetworkScanWifi(interfaceName);
					if (!resp.ok)
						return json(result(opId, step, 'failed', true, resp.error || 'Wi-Fi scan failed'));
					const networks = Array.isArray(resp.payload?.networks) ? resp.payload.networks : [];
					return json(result(opId, step, 'succeeded', false, JSON.stringify(networks)));
				} catch (e: any) {
					return json(result(opId, step, 'failed', true, e.message || 'Wi-Fi scan failed'));
				}
			}

			case 'network.connect_wifi': {
				const interfaceName = payload.interface as string;
				const ssid = payload.ssid as string;
				if (!interfaceName || !/^[a-zA-Z0-9_.-]+$/.test(interfaceName) || !ssid) {
					return json(result(opId, step, 'failed', true, 'Wi-Fi interface and SSID are required'));
				}
				const securityError = validateWifiSecurity(payload.security);
				if (securityError) return json(result(opId, step, 'failed', true, securityError));
				try {
					const resp = await tetraNetworkConnectWifi(
						interfaceName,
						ssid,
						payload.security as Record<string, unknown>
					);
					if (!resp.ok)
						return json(
							result(opId, step, 'failed', true, resp.error || 'Wi-Fi connection failed')
						);
					return json(result(opId, step, 'succeeded', false));
				} catch (e: any) {
					return json(result(opId, step, 'failed', true, e.message || 'Wi-Fi connection failed'));
				}
			}

			case 'network.set_config': {
				const validationError = validateNetworkConfig(payload);
				if (validationError) return json(result(opId, step, 'failed', true, validationError));
				try {
					const resp = await tetraNetworkSetConfig(payload);
					if (!resp.ok)
						return json(
							result(opId, step, 'failed', true, resp.error || 'Network configuration failed')
						);
					return json(result(opId, step, 'succeeded', false));
				} catch (e: any) {
					return json(
						result(opId, step, 'failed', true, e.message || 'Network configuration failed')
					);
				}
			}

			case 'network.check': {
				let resp: Awaited<ReturnType<typeof tetraNetworkStatus>>;
				try {
					resp = await tetraNetworkStatus();
				} catch (e: any) {
					return json(result(opId, step, 'failed', true, e.message || 'Network check failed'));
				}
				if (!resp.ok) {
					return json(result(opId, step, 'failed', true, resp.error || 'Network check failed'));
				}
				return json(result(opId, step, 'succeeded', false, 'Network is available'));
			}

			case 'user.create': {
				const name = payload.name as string;
				if (!name || !/^[a-z_][a-z0-9_-]*$/.test(name)) {
					return json(result(opId, step, 'failed', true, 'Invalid username'));
				}
				try {
					const resp = await tetraCreateUser(name, undefined, undefined);
					if (!resp.ok) throw new Error(resp.error || 'Tetra could not create user');
				} catch (tetraError: any) {
					try {
						await execFileAsync('useradd', ['-m', '-G', 'wheel', name]);
					} catch (e: any) {
						return json(
							result(opId, step, 'failed', true, e.stderr || e.message || tetraError.message)
						);
					}
				}
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
				try {
					const passwordHash = await hashPassword(plaintext);
					const resp = await tetraSetPassword(name, passwordHash);
					if (!resp.ok)
						return json(
							result(opId, step, 'failed', true, resp.error || 'Tetra could not set password')
						);
				} catch (e: any) {
					return json(result(opId, step, 'failed', true, e.message || 'Failed to set password'));
				}
				return json(result(opId, step, 'succeeded', false));
			}

			case 'tetra.detect': {
				let resp: Awaited<ReturnType<typeof tetraCapabilities>>;
				try {
					resp = await tetraCapabilities();
				} catch (e: any) {
					state.tetra = { installed: false, running: false, paired: false };
					await saveState(state);
					return json(result(opId, step, 'succeeded', true, e.message || 'Tetra is not available'));
				}
				const installed = resp.ok && !resp.error;
				state.tetra.installed = installed;
				if (installed) {
					const svc = await tetraServiceStatus('tetra.service');
					state.tetra.running = svc.ok && !svc.error;
				}
				await saveState(state);
				return json(
					result(
						opId,
						step,
						'succeeded',
						false,
						installed ? 'Tetra detected' : 'Tetra not installed'
					)
				);
			}

			case 'tetra.start': {
				const startResp = await tetraServiceStart('tetra.service');
				if (!startResp.ok) {
					return json(
						result(opId, step, 'failed', true, startResp.error || 'Failed to start Tetra')
					);
				}
				const enableResp = await tetraServiceEnable('tetra.service');
				if (!enableResp.ok) {
					return json(
						result(opId, step, 'failed', true, enableResp.error || 'Failed to enable Tetra')
					);
				}
				state.tetra.running = true;
				await saveState(state);
				return json(result(opId, step, 'succeeded', false));
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
				try {
					const args = variant ? [layout, variant] : [layout];
					await execFileAsync('localectl', ['set-keymap', ...args]);
					state.keyboardLayout = layout;
					state.keyboardVariant = variant || undefined;
					await saveState(state);
					return json(result(opId, step, 'succeeded', false));
				} catch (e: any) {
					return json(
						result(opId, step, 'failed', true, e.stderr || e.message || 'Keyboard setup failed')
					);
				}
			}

			case 'ssh-key.apply': {
				const publicKey = typeof payload.publicKey === 'string' ? payload.publicKey.trim() : '';
				if (
					!/^(ssh-(rsa|ed25519|dss)|ecdsa-sha2-nistp\d+|sk-ssh-)[^\s]+\s+[^\s]+/.test(publicKey)
				) {
					return json(result(opId, step, 'failed', true, 'Invalid SSH public key'));
				}
				try {
					const home = (
						await execFileAsync('getent', ['passwd', state.administrator])
					).stdout.split(':')[5];
					const sshDir = `${home}/.ssh`;
					await mkdir(sshDir, { recursive: true, mode: 0o700 });
					await writeFile(`${sshDir}/authorized_keys`, `${publicKey}\n`, { mode: 0o600 });
					await execFileAsync('chown', ['-R', `${state.administrator}:`, sshDir]);
					state.sshPublicKey = publicKey;
					await saveState(state);
					return json(result(opId, step, 'succeeded', false));
				} catch (e: any) {
					return json(
						result(opId, step, 'failed', true, e.stderr || e.message || 'Failed to install SSH key')
					);
				}
			}

			case 'tweaks.apply': {
				const ssh = payload.ssh as boolean | undefined;
				const firewall = payload.firewall as boolean | undefined;
				const autoUpdates = payload.automaticUpdates as boolean | undefined;
				try {
					if (ssh) {
						await execFileAsync('systemctl', ['enable', '--now', 'sshd']);
					}
					if (firewall) {
						await execFileAsync('firewall-cmd', ['--permanent', '--add-service=ssh']);
						await execFileAsync('firewall-cmd', ['--reload']);
					}
					if (autoUpdates) {
						await execFileAsync('systemctl', ['enable', '--now', 'dnf-automatic.timer']);
					}
					state.tweaks = {
						ssh: Boolean(ssh),
						firewall: Boolean(firewall),
						automaticUpdates: Boolean(autoUpdates)
					};
					await saveState(state);
					return json(result(opId, step, 'succeeded', false));
				} catch (e: any) {
					return json(
						result(opId, step, 'failed', true, e.stderr || e.message || 'Tweaks apply failed')
					);
				}
			}

			case 'fyra.begin': {
				const dashboardUrl =
					typeof payload.dashboardUrl === 'string' ? payload.dashboardUrl.trim() : '';
				const agentUrl = typeof payload.agentUrl === 'string' ? payload.agentUrl.trim() : '';
				if (!dashboardUrl) {
					return json(result(opId, step, 'failed', true, 'Dashboard URL is required'));
				}
				try {
					const dashboard = new URL(dashboardUrl);
					if (!['http:', 'https:'].includes(dashboard.protocol))
						throw new Error('Dashboard URL must use HTTP or HTTPS');
					if (agentUrl && !/^wss?:\/\//.test(agentUrl))
						throw new Error('Agent URL must use ws:// or wss://');
				} catch (e) {
					return json(
						result(
							opId,
							step,
							'failed',
							true,
							e instanceof Error ? e.message : 'Invalid enrollment URL'
						)
					);
				}
				const approvalFile = `/run/ultramarine-server-oobe/tetra-enrollment-${opId}.json`;
				try {
					await mkdir('/run/ultramarine-server-oobe', { recursive: true, mode: 0o700 });
					const args = ['enroll', '--dashboard-url', dashboardUrl, '--approval-file', approvalFile];
					if (agentUrl) args.push('--agent-url', agentUrl);
					const child = spawn(env.TETRA_PATH || '/usr/bin/tetra', args, {
						detached: true,
						stdio: 'ignore'
					});
					child.unref();
				} catch (e) {
					return json(
						result(
							opId,
							step,
							'failed',
							true,
							e instanceof Error ? e.message : 'Failed to start Tetra enrollment'
						)
					);
				}
				let approval: { verification_uri?: string; user_code?: string } | undefined;
				for (let attempt = 0; attempt < 20; attempt += 1) {
					try {
						approval = JSON.parse(await readFile(approvalFile, 'utf8')) as typeof approval;
						break;
					} catch {
						await delay(250);
					}
				}
				state.fyra.status = 'pending';
				state.fyra.verificationUri = approval ? approval.verification_uri : undefined;
				state.fyra.userCode = approval ? approval.user_code : undefined;
				state.fyra.message = approval
					? 'Open the verification URL and approve this host.'
					: `Enrollment started. Approval details are in ${approvalFile}.`;
				await saveState(state);
				return json(result(opId, step, 'succeeded', true, state.fyra.message));
			}

			case 'system.reboot': {
				try {
					await execFileAsync('systemctl', ['reboot']);
					return json(result(opId, step, 'succeeded', false, 'Rebooting…'));
				} catch (e: any) {
					return json(
						result(opId, step, 'failed', true, e.stderr || e.message || 'Failed to reboot')
					);
				}
			}

			case 'system.poweroff': {
				try {
					await execFileAsync('systemctl', ['poweroff']);
					return json(result(opId, step, 'succeeded', false, 'Shutting down…'));
				} catch (e: any) {
					return json(
						result(opId, step, 'failed', true, e.stderr || e.message || 'Failed to shut down')
					);
				}
			}

			case 'dashboard.install': {
				const buildPath =
					env.DASHBOARD_BUILD_PATH || '/usr/lib/ultramarine-dashboard/apps/dashboard/build';
				const serviceName = env.DASHBOARD_SERVICE_NAME || 'ultramarine-dashboard.service';

				try {
					await access(buildPath, constants.R_OK);
				} catch {
					return json(
						result(opId, step, 'failed', true, `Dashboard build not found at ${buildPath}`)
					);
				}

				const unitPath = `/etc/systemd/system/${serviceName}`;
				const unitContent = `[Unit]
Description=Ultramarine Server Dashboard
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
WorkingDirectory=${buildPath}
ExecStart=/usr/bin/node ${buildPath}
Restart=on-failure
Environment="PORT=3972"
Environment="ORIGIN=http://localhost:3972"
Environment="HOST=0.0.0.0"

[Install]
WantedBy=multi-user.target
`;

				try {
					await writeFile(unitPath, unitContent, 'utf-8');
					await execFileAsync('systemctl', ['daemon-reload']);
					await execFileAsync('systemctl', ['enable', serviceName]);
				} catch (e: any) {
					return json(
						result(
							opId,
							step,
							'failed',
							true,
							e.stderr || e.message || 'Failed to install dashboard service'
						)
					);
				}

				if (!state.dashboard) {
					state.dashboard = { installed: false, installing: false };
				}
				state.dashboard.installed = true;
				state.dashboard.installing = false;
				await saveState(state);
				return json(result(opId, step, 'succeeded', false, 'Dashboard service installed'));
			}

			case 'dashboard.handoff': {
				const oobeService = env.OOBE_SERVICE_NAME || 'ultramarine-server-oobe.service';
				const dashboardService = env.DASHBOARD_SERVICE_NAME || 'ultramarine-dashboard.service';

				const scriptPath = `/tmp/ultramarine-dashboard-handoff-${Date.now()}.sh`;
				const script = `#!/bin/sh
set -e
sleep 3
systemctl stop ${oobeService} 2>/dev/null || true
systemctl start ${dashboardService}
rm -f ${scriptPath}
`;

				try {
					await writeFile(scriptPath, script, { mode: 0o755 });
					const child = spawn('sh', [scriptPath], {
						detached: true,
						stdio: 'ignore'
					});
					child.unref();
				} catch (e: any) {
					return json(
						result(opId, step, 'failed', true, e.message || 'Failed to schedule handoff')
					);
				}

				return json(result(opId, step, 'succeeded', false, 'Handoff scheduled'));
			}

			case 'cloudflare.install': {
				const cloudflaredBin = env.CLOUDFLARED_PATH || '/usr/bin/cloudflared';
				const serviceName = env.CLOUDFLARED_SERVICE_NAME || 'ultramarine-cloudflared.service';
				const tokenDir = env.CLOUDFLARED_TOKEN_DIR || '/var/lib/ultramarine/cloudflared';
				const tokenFile = `${tokenDir}/token`;
				const wrapperPath = '/usr/local/bin/ultramarine-cloudflared-wrapper.sh';

				// Try to install cloudflared if not present
				try {
					await access(cloudflaredBin, constants.X_OK);
				} catch {
					try {
						await execFileAsync('dnf', ['install', '-y', 'cloudflared']);
					} catch {
						// Fall back to official RPM install path
						try {
							await execFileAsync('dnf', [
								'install',
								'-y',
								'https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-x86_64.rpm'
							]);
						} catch (e: any) {
							return json(
								result(
									opId,
									step,
									'failed',
									true,
									e.stderr || e.message || 'Failed to install cloudflared'
								)
							);
						}
					}
				}

				// Ensure token directory exists
				try {
					await mkdir(tokenDir, { recursive: true });
				} catch {
					// ignore
				}

				// Create wrapper script that reads token from file
				const wrapperScript = `#!/bin/sh
set -e
if [ -r "${tokenFile}" ]; then
  export TUNNEL_TOKEN=$(cat "${tokenFile}")
fi
exec ${cloudflaredBin} tunnel run
`;
				try {
					await writeFile(wrapperPath, wrapperScript, { mode: 0o755 });
				} catch (e: any) {
					return json(
						result(opId, step, 'failed', true, e.message || 'Failed to write wrapper script')
					);
				}

				const unitPath = `/etc/systemd/system/${serviceName}`;
				const unitContent = `[Unit]
Description=Cloudflare Tunnel for Ultramarine Server
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart=${wrapperPath}
Restart=on-failure

[Install]
WantedBy=multi-user.target
`;

				try {
					await writeFile(unitPath, unitContent, 'utf-8');
					await execFileAsync('systemctl', ['daemon-reload']);
					await execFileAsync('systemctl', ['enable', serviceName]);
				} catch (e: any) {
					return json(
						result(
							opId,
							step,
							'failed',
							true,
							e.stderr || e.message || 'Failed to install cloudflared service'
						)
					);
				}

				if (!state.cloudflare) {
					state.cloudflare = { installed: false, installing: false };
				}
				state.cloudflare.installed = true;
				state.cloudflare.installing = false;
				await saveState(state);
				return json(
					result(
						opId,
						step,
						'succeeded',
						false,
						'Cloudflared installed. Place tunnel token in ' + tokenFile + ' and start the service.'
					)
				);
			}

			default:
				return json(result(opId, step, 'failed', false, `Unknown operation: ${operation}`));
		}
	} catch (err) {
		console.error('Operation failed:', err);
		throw error(500, 'Internal server error');
	}
};
