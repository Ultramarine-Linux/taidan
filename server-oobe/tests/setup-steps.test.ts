// The network (addressing, safe apply, Wi-Fi and 802.1X) and the update
// policy, remote access and dashboard access steps, at the API (against the
// mock Tetra in fixture mode) and in the browser. Network operations in the
// browser also go to the mock Tetra, on the fixture host the
// `oobe_fixture_host` cookie picks.
import { expect, localURL, test } from './accessibility';
import type { APIRequestContext, Page } from '@playwright/test';
import { FIXTURE_SETUP_CODE } from '../src/lib/fixture';
import { FIXTURE_HOST_COOKIE, type FixtureHostName } from '../src/lib/fixture-hosts';

async function login(request: APIRequestContext, host?: FixtureHostName) {
	const res = await request.post(`${localURL}/api/oobe/session`, {
		data: { code: FIXTURE_SETUP_CODE },
		headers: host ? { cookie: `${FIXTURE_HOST_COOKIE}=${host}` } : {}
	});
	expect(res.status()).toBe(200);
}

let changes = 0;
/** A change id for network.configure and network.connect_wifi. */
function newChangeId() {
	changes += 1;
	return `test-${process.pid}-${Date.now()}-${changes}`;
}

async function confirm(request: APIRequestContext, changeId: string) {
	const res = await request.post(`${localURL}/api/oobe/network/confirm`, { data: { changeId } });
	return (await res.json()) as { status: string; message?: string };
}

async function op(
	request: APIRequestContext,
	step: string,
	operation: string,
	payload: Record<string, unknown>
) {
	const res = await request.post(`${localURL}/api/oobe/operations`, {
		data: { step, operation, payload }
	});
	expect(res.status()).toBe(200);
	return (await res.json()) as { status: string; message?: string };
}

test.describe('setup operations', () => {
	test.beforeEach(async ({ request }) => login(request));

	test('removed host operations are rejected', async ({ request }) => {
		for (const operation of [
			'dashboard.install',
			'dashboard.handoff',
			'cloudflare.install',
			'tweaks.apply',
			'tetra.start',
			'fyra.begin',
			'network.set_config'
		]) {
			expect(await op(request, 'dashboard', operation, {}), operation).toMatchObject({
				status: 'failed',
				message: 'Unsupported setup operation'
			});
		}
	});

	test('Wi-Fi security uses Tetra field names', async ({ request }) => {
		const connect = (security: Record<string, unknown>) =>
			op(request, 'internet', 'network.connect_wifi', {
				changeId: newChangeId(),
				interface: 'wlan0',
				ssid: 'Corp Wi-Fi',
				security
			});
		expect(
			await connect({ type: 'wpa-enterprise', eap: 'peap', username: 'alice', password: 'p' })
		).toMatchObject({ status: 'failed', message: 'Unknown 802.1X setting: username' });
		expect(
			await connect({
				type: 'wpa-enterprise',
				eap: 'peap',
				identity: 'alice',
				anonymous_identity: 'anonymous',
				password: 'p',
				domain_suffix_match: 'radius.example.com',
				phase2: 'mschapv2'
			})
		).toMatchObject({ status: 'succeeded' });
		expect(
			await connect({
				type: 'wpa-enterprise',
				eap: 'peap',
				identity: 'a',
				password: 'p',
				ca_cert: '/etc/ca.pem'
			})
		).toMatchObject({ status: 'failed', message: 'CA certificate must be a PEM file' });
		expect(await connect({ type: 'wpa-personal', password: 'p', extra: 1 })).toMatchObject({
			status: 'succeeded'
		});
		expect(await connect({ type: 'open' })).toMatchObject({ status: 'succeeded' });
	});

	test('network.configure checks the change before Tetra sees it', async ({ request }) => {
		const configure = (payload: Record<string, unknown>) =>
			op(request, 'internet', 'network.configure', { changeId: newChangeId(), ...payload });
		expect(
			await op(request, 'internet', 'network.configure', { interface: 'enp1s0' })
		).toMatchObject({ status: 'failed', message: 'Missing change id' });
		// Fewer than 128 bits.
		expect(
			await op(request, 'internet', 'network.configure', {
				changeId: 'abcdefghijklmnopqrstu',
				interface: 'enp1s0'
			})
		).toMatchObject({ status: 'failed', message: 'Missing change id' });
		expect(
			await configure({
				interface: 'enp1s0',
				ipv4: { method: 'disabled' },
				ipv6: { method: 'disabled' }
			})
		).toMatchObject({ status: 'failed', message: 'IPv4 and IPv6 cannot both be off' });
		expect(
			await configure({
				interface: 'enp1s0',
				ipv4: { method: 'static', address: '192.0.2.10', prefix_length: 24 }
			})
		).toMatchObject({ status: 'failed', message: 'Unknown IPv4 setting: address' });
		// Without sections Tetra uses automatic addressing for both families.
		expect(await configure({ interface: 'enp1s0' })).toMatchObject({
			status: 'succeeded',
			message: JSON.stringify({ confirmationRequired: false })
		});
	});

	test('network.check reports addresses per family and DNS', async ({ request }) => {
		const check = await op(request, 'internet', 'network.check', {});
		expect(JSON.parse(check.message ?? '{}')).toEqual({ ipv4: true, ipv6: true, dns: true });
	});

	test('update policy accepts only automatic or notify', async ({ request }) => {
		expect(await op(request, 'updates', 'updates.set_policy', { policy: 'notify' })).toMatchObject({
			status: 'succeeded'
		});
		expect(await op(request, 'updates', 'updates.set_policy', { policy: 'yolo' })).toMatchObject({
			status: 'failed'
		});
	});

	test('Tailscale Funnel needs explicit confirmation', async ({ request }) => {
		const base = { auth_key: 'tskey-auth-secret123', serve: true, funnel: true };
		expect(await op(request, 'remote-access', 'remote_access.tailscale_up', base)).toMatchObject({
			status: 'failed',
			message: 'Confirm that the dashboard will be public'
		});
		expect(
			await op(request, 'remote-access', 'remote_access.tailscale_up', {
				...base,
				confirmFunnel: true
			})
		).toMatchObject({ status: 'succeeded' });
		expect(
			await op(request, 'remote-access', 'remote_access.tailscale_up', {
				auth_key: 'not-a-key',
				serve: true,
				funnel: false
			})
		).toMatchObject({ status: 'failed' });
	});

	test('access.set validates the mode and DNS-01 credentials', async ({ request }) => {
		expect(
			await op(request, 'dashboard', 'access.set', { mode: 'ip', hostname: '192.0.2.10' })
		).toMatchObject({
			status: 'succeeded',
			message: JSON.stringify({ passkeysAvailable: false, dashboardUrl: 'https://192.0.2.10' })
		});
		expect(
			await op(request, 'dashboard', 'access.set', { mode: 'ip', hostname: 'example.com' })
		).toMatchObject({ status: 'failed' });
		expect(
			await op(request, 'dashboard', 'access.set', {
				mode: 'domain',
				hostname: 'server.example.com',
				challenge: 'dns-01',
				dns_provider: 'cloudflare',
				dns_credentials: {}
			})
		).toMatchObject({ status: 'failed', message: 'API token is required' });
		expect(
			await op(request, 'dashboard', 'access.set', {
				mode: 'domain',
				hostname: 'server.example.com',
				challenge: 'dns-01',
				dns_provider: 'cloudflare',
				dns_credentials: { api_token: 'x', extra: 'y' }
			})
		).toMatchObject({ status: 'failed' });
	});

	test('secrets are passed to Tetra and never stored', async ({ request }) => {
		const secrets = ['tskey-auth-neverstored', 'cf-tunnel-neverstored', 'dns-neverstored'];
		await op(request, 'remote-access', 'remote_access.tailscale_up', {
			auth_key: secrets[0],
			serve: true,
			funnel: false
		});
		await op(request, 'remote-access', 'remote_access.cloudflared_set_token', {
			token: secrets[1]
		});
		expect(
			await op(request, 'dashboard', 'access.set', {
				mode: 'domain',
				hostname: 'server.example.com',
				challenge: 'dns-01',
				dns_provider: 'cloudflare',
				dns_credentials: { api_token: secrets[2] }
			})
		).toMatchObject({ status: 'succeeded' });

		const state = await (await request.get(`${localURL}/api/oobe/state`)).text();
		for (const secret of secrets) expect(state).not.toContain(secret);
		expect(JSON.parse(state).access).toMatchObject({
			mode: 'domain',
			hostname: 'server.example.com',
			challenge: 'dns-01',
			dnsProvider: 'cloudflare'
		});

		const status = await op(request, 'remote-access', 'remote_access.status', {});
		expect(JSON.parse(status.message ?? '{}')).toMatchObject({
			tailscale: { enabled: true, serve: true, funnel: false },
			cloudflared: { enabled: true }
		});
	});
});

test.describe('network on an IPv6-only host', () => {
	test.beforeEach(async ({ request }) => login(request, 'ipv6-only'));

	test('manual IPv6 is applied and kept after network.confirm', async ({ request }) => {
		const interfaces = await op(request, 'internet', 'network.interfaces', {});
		expect(JSON.parse(interfaces.message ?? '[]')).toEqual([
			{
				name: 'enp1s0',
				type: 'ethernet',
				operstate: 'up',
				addresses: [
					expect.objectContaining({ address: '2001:db8:10::2', scope: 'global' }),
					expect.objectContaining({ family: 'ipv6', scope: 'link-local' })
				]
			}
		]);
		const check = await op(request, 'internet', 'network.check', {});
		expect(JSON.parse(check.message ?? '{}')).toEqual({ ipv4: false, ipv6: true, dns: true });

		const changeId = newChangeId();
		expect(
			await op(request, 'internet', 'network.configure', {
				changeId,
				interface: 'enp1s0',
				ipv4: { method: 'disabled' },
				ipv6: {
					method: 'manual',
					addresses: [{ address: '2001:db8:10::20', prefix_length: 64 }],
					gateway: 'fe80::1',
					dns: ['2606:4700:4700::1111']
				}
			})
		).toMatchObject({
			status: 'succeeded',
			message: JSON.stringify({ confirmationRequired: true, rollbackInSeconds: 60 })
		});
		// Tetra holds one change at a time.
		expect(
			await op(request, 'internet', 'network.configure', {
				changeId: newChangeId(),
				interface: 'enp1s0'
			})
		).toMatchObject({
			status: 'failed',
			message: expect.stringContaining('awaiting confirmation')
		});

		expect(await confirm(request, changeId)).toEqual({ status: 'kept' });
		const after = await op(request, 'internet', 'network.interfaces', {});
		expect(after.message).toContain('2001:db8:10::20');
		expect(after.message).not.toContain('2001:db8:10::2"');
		expect(
			await op(request, 'dashboard', 'access.set', { mode: 'ip', hostname: '[2001:db8:10::20]' })
		).toMatchObject({
			status: 'succeeded',
			message: JSON.stringify({
				passkeysAvailable: false,
				dashboardUrl: 'https://[2001:db8:10::20]'
			})
		});
	});
});

test.describe('network on a Wi-Fi host', () => {
	test.beforeEach(async ({ request }) => login(request, 'wifi'));

	test('scan_wifi and connect_wifi', async ({ request }) => {
		const scan = await op(request, 'internet', 'network.scan_wifi', { interface: 'wlp2s0' });
		expect(JSON.parse(scan.message ?? '[]')).toContainEqual({
			ssid: 'Ultramarine-Setup',
			security: 'wpa-personal',
			signal: 86,
			frequency: 5180,
			in_use: true
		});
		const changeId = newChangeId();
		expect(
			await op(request, 'internet', 'network.connect_wifi', {
				changeId,
				interface: 'wlp2s0',
				ssid: 'Hidden Lab',
				hidden: true,
				security: { type: 'wpa-personal', password: 'correct horse' }
			})
		).toMatchObject({
			status: 'succeeded',
			message: JSON.stringify({ confirmationRequired: false })
		});
		expect(await confirm(request, changeId)).toEqual({ status: 'kept' });
		expect(await op(request, 'internet', 'network.scan_wifi', { interface: 'lo' })).toMatchObject({
			status: 'failed'
		});
	});
});

/** Pick the mock Tetra's fixture host for this browser (before logging in). */
async function useHost(page: Page, host: FixtureHostName) {
	await page.context().addCookies([{ name: FIXTURE_HOST_COOKIE, value: host, url: localURL }]);
}

/** Enter the setup code and continue until `title` is showing. */
async function walkTo(page: Page, title: string) {
	await page.goto(localURL);
	await page.locator('#setup-code').fill(FIXTURE_SETUP_CODE);
	const heading = page.locator('.workspace h2');
	const next = page.locator('button.primary-button');
	for (let i = 0; i < 12 && (await heading.textContent()) !== title; i += 1) {
		if (await page.locator('#device-name').isVisible()) {
			await page.locator('#device-name').fill('test-device');
			await page.locator('#hostname').fill('test-device');
		}
		if (await page.locator('#username').isVisible()) {
			await page.locator('#full-name').fill('Test User');
			await page.locator('#username').fill('testuser');
		}
		if (await page.locator('#password-confirm').isVisible()) {
			await page.locator('#password').fill('securepassword123');
			await page.locator('#password-confirm').fill('securepassword123');
		}
		const before = await heading.textContent();
		await next.click();
		await expect(heading).not.toHaveText(before ?? '');
	}
	await expect(heading).toHaveText(title);
}

test.describe('setup steps', () => {
	test('wired 802.1X asks for Tetra settings by EAP method', async ({ page }) => {
		await walkTo(page, 'Configure your network');
		await page.locator('#network-interface').selectOption('enp1s0');
		await expect(page.locator('#wired-8021x')).toHaveCount(0);
		await page.locator('#network-configure').check();
		await page.locator('#wired-8021x').check();
		await expect(page.locator('#wired-identity')).toBeVisible();
		await expect(page.locator('#wired-anonymous-identity')).toBeVisible();
		await expect(page.locator('#wired-password')).toBeVisible();
		await expect(page.locator('#wired-domain')).toBeVisible();
		await expect(page.locator('#wired-clientCert-upload')).toHaveCount(0);

		await page.locator('#wired-eap').selectOption('tls');
		await expect(page.locator('#wired-password')).toHaveCount(0);
		await expect(page.locator('#wired-phase2')).toHaveCount(0);
		await expect(page.locator('#wired-anonymous-identity')).toHaveCount(0);
		await expect(page.locator('#wired-private-key-password')).toBeVisible();

		await page.locator('#wired-privateKey-upload').setInputFiles({
			name: 'client.der',
			mimeType: 'application/octet-stream',
			buffer: Buffer.from([0x30, 0x82, 0x01, 0x0a])
		});
		await expect(page.getByRole('alert')).toContainText('Choose a PEM file');
		await page.locator('#wired-privateKey-upload').setInputFiles({
			name: 'client.key',
			mimeType: 'application/x-pem-file',
			buffer: Buffer.from('-----BEGIN PRIVATE KEY-----\nMIIE\n-----END PRIVATE KEY-----\n')
		});
		await expect(page.getByRole('alert')).toHaveCount(0);
		await expect(page.getByText('client.key')).toBeVisible();
	});

	test('enterprise Wi-Fi asks for an identity, not a username', async ({ page }) => {
		await walkTo(page, 'Configure your network');
		await page.locator('#network-interface').selectOption('wlan0');
		await page.getByRole('button', { name: 'Scan for networks' }).click();
		await page.locator('#wifi-network').selectOption('Corp Wi-Fi');
		await expect(page.locator('#wifi-identity')).toBeVisible();
		await expect(page.locator('#wifi-domain')).toBeVisible();
		await expect(page.locator('#wifi-password')).toHaveCount(1);
		await expect(page.locator('#wifi-username')).toHaveCount(0);
		await page.locator('#wifi-eap').selectOption('tls');
		await expect(page.locator('#wifi-password')).toHaveCount(0);
		await expect(page.locator('#wifi-clientCert-upload')).toHaveCount(1);
	});

	test('IPv6-only host: manual IPv6 survives losing the connection', async ({ page }) => {
		await useHost(page, 'ipv6-only');
		await walkTo(page, 'Configure your network');
		await expect(page.locator('.network-current-addresses')).toContainText('2001:db8:10::2/64');
		await expect(page.locator('#ipv4-auto')).toHaveCount(0);
		await page.locator('#network-configure').check();
		await expect(page.locator('#ipv4-auto')).toBeChecked();
		await expect(page.locator('#ipv6-auto')).toBeChecked();

		const next = page.locator('button.primary-button');
		await page.locator('#ipv4-disabled').check();
		await page.locator('#ipv6-disabled').check();
		await expect(page.getByRole('alert')).toContainText("IPv4 and IPv6 can't both be off");
		await expect(next).toBeDisabled();

		await page.locator('#ipv6-manual').check();
		await page.locator('#ipv6-addresses').fill('2001:db8:10::20/64');
		await page.locator('#ipv6-gateway').fill('fe80::1');
		await page.locator('#ipv6-dns').fill('2606:4700:4700::1111');
		await expect(next).toBeEnabled();

		// The server applies the change, but the browser never hears back, and
		// can't reach the OOBE for the first few tries.
		await page.route('**/api/oobe/operations', async (route) => {
			if (route.request().postDataJSON()?.operation !== 'network.configure') {
				return route.continue();
			}
			await route.fetch();
			await route.abort('connectionreset');
		});
		let unreachable = 2;
		await page.route('**/api/oobe/network/confirm', (route) =>
			unreachable-- > 0 ? route.abort('addressunreachable') : route.continue()
		);
		await next.click();

		const apply = page.locator('.network-apply');
		await expect(apply).toContainText('Waiting to reach setup again');
		await expect(apply.getByRole('link')).toHaveText(['https://[2001:db8:10::20]:6623']);
		await expect(apply.getByRole('link')).toHaveAttribute(
			'href',
			/^https:\/\/\[2001:db8:10::20\]:6623\/\?network-change=[\w-]+$/
		);
		// Fluent isolates the number with U+2068 and U+2069.
		await expect(apply).toContainText(/within \W?60\W? seconds/);
		await expect(page.locator('.workspace h2')).toHaveText('Choose an update policy');
	});

	test('a change nobody confirms is reported as rolled back', async ({ page }) => {
		await useHost(page, 'ipv6-only');
		await walkTo(page, 'Configure your network');
		await page.locator('#network-configure').check();
		await page.locator('#ipv6-manual').check();
		await page.locator('#ipv6-addresses').fill('2001:db8:10::21/64');
		await page.route('**/api/oobe/network/confirm', (route) =>
			route.fulfill({ json: { status: 'rolled-back' } })
		);
		await page.locator('button.primary-button').click();
		await expect(page.locator('.network-apply')).toContainText(
			'so the server went back to its previous network settings'
		);
		await expect(page.locator('.workspace h2')).toHaveText('Configure your network');
	});

	test('Wi-Fi host: addressing is automatic', async ({ page }) => {
		await useHost(page, 'wifi');
		await walkTo(page, 'Configure your network');
		await expect(page.locator('#network-interface')).toHaveValue('wlp2s0');
		await expect(
			page.getByText('Wi-Fi gets its IPv4 and IPv6 addresses automatically.')
		).toBeVisible();
		await expect(page.locator('#network-configure')).toHaveCount(0);
		await expect(page.locator('#ipv4-auto')).toHaveCount(0);

		await page.getByRole('button', { name: 'Scan for networks' }).click();
		await page.locator('#wifi-network').selectOption('Ultramarine-Setup');
		await page.locator('#wifi-password').fill('correct horse');
		await page.getByRole('button', { name: 'Connect', exact: true }).click();
		await expect(page.locator('.network-apply')).toContainText('Network settings applied.');
	});

	test('only Ethernet interfaces offer wired settings', async ({ page }) => {
		await walkTo(page, 'Configure your network');
		await page.locator('#network-interface').selectOption('virbr0');
		await expect(page.locator('.network-other-interface')).toBeVisible();
		await expect(page.locator('#network-configure')).toHaveCount(0);
		await expect(page.getByRole('button', { name: 'Scan for networks' })).toHaveCount(0);
		await page.locator('#network-interface').selectOption('enp1s0');
		await expect(page.locator('#network-configure')).toBeVisible();
	});

	test('without a type from Tetra, wl* interfaces are Wi-Fi', async ({ page }) => {
		// An older Tetra that does not report `type`.
		await page.route('**/api/oobe/operations', async (route) => {
			if (route.request().postDataJSON()?.operation !== 'network.interfaces') {
				return route.continue();
			}
			await route.fulfill({
				json: {
					id: 'op',
					step: 'internet',
					status: 'succeeded',
					retryable: false,
					message: JSON.stringify([
						{ name: 'eth0', operstate: 'up', addresses: [] },
						{ name: 'wlan1', operstate: 'down', addresses: [] }
					])
				}
			});
		});
		await walkTo(page, 'Configure your network');
		await expect(page.locator('#network-interface')).toHaveValue('eth0');
		await expect(page.locator('#network-configure')).toBeVisible();
		await page.locator('#network-interface').selectOption('wlan1');
		await expect(page.getByRole('button', { name: 'Scan for networks' })).toBeVisible();
		await expect(page.locator('#network-configure')).toHaveCount(0);
	});

	test('check connection reports each family and DNS', async ({ page }) => {
		await useHost(page, 'ipv6-only');
		await walkTo(page, 'Configure your network');
		await page.getByRole('button', { name: 'Check connection' }).click();
		await expect(page.locator('.network-check li')).toHaveText([
			'No routable IPv4 address.',
			'This server has an IPv6 address others can reach.',
			'DNS resolves names.'
		]);
	});

	test('IP mode offers the IPv6 address, bracketed in the URL', async ({ page }) => {
		await useHost(page, 'ipv6-only');
		await walkTo(page, 'Choose how to reach the Dashboard');
		await page.locator('#access-ip').check();
		await expect(page.locator('#access-ip-address')).toHaveValue('2001:db8:10::2');
		await expect(page.locator('.dashboard-preview strong')).toHaveText('https://[2001:db8:10::2]');
	});

	test('update policy defaults to automatic', async ({ page }) => {
		await walkTo(page, 'Choose an update policy');
		await expect(page.locator('#updates-automatic')).toBeChecked();
		await expect(page.locator('#updates-notify')).not.toBeChecked();
	});

	test('remote access is off by default and Funnel warns first', async ({ page }) => {
		await walkTo(page, 'Set up remote access');
		const next = page.locator('button.primary-button');
		await expect(page.locator('#tailscale-enable')).not.toBeChecked();
		await expect(page.locator('#cloudflared-enable')).not.toBeChecked();
		await expect(next).toBeEnabled();

		await page.locator('#tailscale-enable').check();
		await expect(page.locator('#tailscale-serve')).toBeChecked();
		await expect(page.locator('#tailscale-funnel')).not.toBeChecked();
		await page.locator('#tailscale-auth-key').fill('tskey-auth-abc');
		await expect(next).toBeEnabled();

		await page.locator('#tailscale-funnel').check();
		await expect(page.getByRole('alert')).toContainText('Anyone on the internet');
		await expect(next).toBeDisabled();
		await page.locator('#tailscale-funnel-confirm').check();
		await expect(next).toBeEnabled();
	});

	test('IP mode warns that passkeys are unavailable', async ({ page }) => {
		await walkTo(page, 'Choose how to reach the Dashboard');
		await expect(page.locator('text=Passkeys won')).toHaveCount(0);
		await page.locator('#access-ip').check();
		await expect(page.getByText("Passkeys won't be available")).toBeVisible();
	});

	test('DNS-01 asks for the provider credentials', async ({ page }) => {
		await walkTo(page, 'Choose how to reach the Dashboard');
		const next = page.locator('button.primary-button');
		await page.locator('#access-hostname').fill('server.example.com');
		await expect(next).toBeEnabled();
		await page.locator('#challenge-dns').check();
		await expect(next).toBeDisabled();
		await page.locator('#dns-provider').selectOption('cloudflare');
		await page.locator('#dns-api_token').fill('token');
		await expect(next).toBeEnabled();
	});
});
