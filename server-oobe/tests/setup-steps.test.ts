// The update policy, remote access and dashboard access steps (steps 4-6), at
// the API (against the mock Tetra in fixture mode) and in the browser.
import { expect, localURL, test } from './accessibility';
import type { APIRequestContext, Page } from '@playwright/test';
import { FIXTURE_SETUP_CODE } from '../src/lib/fixture';

async function login(request: APIRequestContext) {
	const res = await request.post(`${localURL}/api/oobe/session`, {
		data: { code: FIXTURE_SETUP_CODE }
	});
	expect(res.status()).toBe(200);
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
			'fyra.begin'
		]) {
			expect(await op(request, 'dashboard', operation, {}), operation).toMatchObject({
				status: 'failed',
				message: 'Unsupported setup operation'
			});
		}
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
