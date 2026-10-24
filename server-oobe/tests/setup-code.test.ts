import { expect, localURL, test } from './accessibility';
import { FIXTURE_SETUP_CODE } from '../src/lib/fixture';

const operation = { step: 'devicename', operation: 'hostname.apply', payload: { hostname: 'x' } };

test.describe('setup code gate', () => {
	test('rejects operations and state access without a setup session', async ({ request }) => {
		const op = await request.post(`${localURL}/api/oobe/operations`, { data: operation });
		expect(op.status()).toBe(401);

		const state = await request.get(`${localURL}/api/oobe/state`);
		expect(state.status()).toBe(401);

		const patch = await request.post(`${localURL}/api/oobe/state`, {
			data: { steps: [{ id: 'welcome', status: 'complete' }] }
		});
		expect(patch.status()).toBe(401);
	});

	test('rejects a forged session cookie', async ({ request }) => {
		const res = await request.post(`${localURL}/api/oobe/operations`, {
			data: operation,
			headers: { cookie: 'oobe_setup_session=tts_forged' }
		});
		expect(res.status()).toBe(401);
	});

	test('rejects a wrong setup code', async ({ request }) => {
		const res = await request.post(`${localURL}/api/oobe/session`, { data: { code: 'WRNG-CODE' } });
		expect(res.status()).toBe(401);
		expect(await res.json()).toMatchObject({ authenticated: false, error: 'invalid_code' });

		const op = await request.post(`${localURL}/api/oobe/operations`, { data: operation });
		expect(op.status()).toBe(401);
	});

	test('a redeemed code allows operations', async ({ request }) => {
		const before = await request.get(`${localURL}/api/oobe/session`);
		expect(await before.json()).toMatchObject({ authenticated: false, setupRequired: true });

		const res = await request.post(`${localURL}/api/oobe/session`, {
			data: { code: FIXTURE_SETUP_CODE.toLowerCase() }
		});
		expect(res.status()).toBe(200);
		expect(await res.json()).toMatchObject({ authenticated: true });
		const cookie = res.headers()['set-cookie'];
		expect(cookie).toContain('oobe_setup_session=tts_');
		expect(cookie).toContain('HttpOnly');
		expect(cookie).toContain('SameSite=Strict');

		const after = await request.get(`${localURL}/api/oobe/session`);
		expect(await after.json()).toMatchObject({ authenticated: true });

		const op = await request.post(`${localURL}/api/oobe/operations`, { data: operation });
		expect(op.status()).toBe(200);
		expect(await op.json()).toMatchObject({ status: 'succeeded' });
	});

	test('network addresses stay public for the welcome screen', async ({ request }) => {
		const res = await request.get(`${localURL}/api/oobe/network`);
		expect(res.status()).toBe(200);
	});
});

test.describe('welcome step', () => {
	test('asks for the setup code before continuing', async ({ page }) => {
		await page.goto(localURL);
		const code = page.locator('#setup-code');
		const next = page.locator('button.primary-button').filter({ hasText: 'Continue' });

		await expect(code).toBeVisible();
		await expect(next).toBeDisabled();

		await code.fill('WRNG-CODE');
		await next.click();
		await expect(page.locator('#setup-code-error')).toBeVisible();
		await expect(page.locator('.workspace h2')).toHaveText('Welcome to Ultramarine Server');

		await code.fill(FIXTURE_SETUP_CODE);
		await next.click();
		await expect(page.locator('.workspace h2')).toHaveText('Name your server');
	});
});

test.describe('keyboard', () => {
	test('Enter in a field continues once the step allows it', async ({ page }) => {
		await page.goto(localURL);
		const heading = page.locator('.workspace h2');
		const code = page.locator('#setup-code');

		// Nothing to submit yet.
		await code.press('Enter');
		await expect(heading).toHaveText('Welcome to Ultramarine Server');

		await code.fill(FIXTURE_SETUP_CODE);
		await code.press('Enter');
		await expect(heading).toHaveText('Name your server');

		const hostname = page.locator('#hostname');
		await hostname.fill('Not Valid!');
		await hostname.press('Enter');
		await expect(heading).toHaveText('Name your server');

		await hostname.fill('enter-test');
		await hostname.press('Enter');
		await expect(heading).toHaveText('Create a user');
	});
});
