import { expect, localURL, test } from './accessibility';
import type { Page } from '@playwright/test';
import { FIXTURE_SETUP_CODE } from '../src/lib/fixture';

const stepIds = [
	'welcome',
	'devicename',
	'whoareyou',
	'password',
	'ssh-key',
	'internet',
	'updates',
	'remote-access',
	'dashboard',
	'complete'
] as const;

async function resetState(page: Page) {
	// Every API call needs a setup session from the (mock) Tetra.
	await page.request.post(`${localURL}/api/oobe/session`, { data: { code: FIXTURE_SETUP_CODE } });
	await page.request.post(`${localURL}/api/oobe/operations`, {
		data: {
			step: 'welcome',
			operation: 'state.reset',
			payload: { activeStep: 'welcome', completed: false, steps: [] }
		}
	});
}

for (const theme of ['light', 'dark'] as const) {
	test(`Ultramarine ${theme} OOBE steps`, async ({ page: browserPage, makeAxeBuilder }) => {
		await browserPage.addInitScript((selectedTheme) => {
			localStorage.setItem('oobe-theme', selectedTheme);
			document.documentElement.classList.toggle('dark', selectedTheme === 'dark');
		}, theme);

		// Reset persisted state so every run starts at Welcome
		await resetState(browserPage);
		await browserPage.goto(localURL);
		await expect(browserPage.locator('.workspace h2')).toBeVisible();

		for (const stepId of stepIds) {
			// Run axe on the current step before advancing
			const results = await makeAxeBuilder().analyze();
			expect(results.violations, `Accessibility violations on ${stepId}`).toEqual([]);

			// Fill required fields before continuing past form steps
			if (stepId === 'welcome') {
				await browserPage.locator('#setup-code').fill(FIXTURE_SETUP_CODE);
			}
			if (stepId === 'devicename') {
				await browserPage.locator('#device-name').fill('test-device');
				await browserPage.locator('#hostname').fill('test-device');
			}
			if (stepId === 'whoareyou') {
				await browserPage.locator('#full-name').fill('Test User');
				await browserPage.locator('#username').fill('testuser');
			}
			if (stepId === 'dashboard') {
				await browserPage.locator('#access-hostname').fill('server.example.com');
			}
			if (stepId === 'password') {
				await browserPage.locator('#password').fill('securepassword123');
				await browserPage.locator('#password-confirm').fill('securepassword123');
			}

			// Click Continue to move to next step (skip on last step)
			if (stepId !== 'complete') {
				await browserPage
					.locator('button.primary-button')
					.filter({ hasText: /(Continue|Next|Finish|Review setup|Go to Dashboard)/ })
					.click();
				await expect(browserPage.locator('.workspace h2')).toBeVisible();
			}
		}

		// The final screen comes from setup.complete's response.
		await browserPage.locator('button.primary-button').filter({ hasText: 'Finish setup' }).click();
		await expect(browserPage.locator('.workspace h2')).toHaveText('Setup complete');
		const results = await makeAxeBuilder().analyze();
		expect(results.violations, 'Accessibility violations on the final screen').toEqual([]);
	});
}
