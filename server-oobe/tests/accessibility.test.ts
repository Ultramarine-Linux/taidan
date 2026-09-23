import { expect, localURL, test } from './accessibility';
import type { Page } from '@playwright/test';

const stepIds = [
	'welcome',
	'devicename',
	'whoareyou',
	'password',
	'ssh-key',
	'internet',
	'tweaks',
	'dashboard',
	'complete'
] as const;

async function resetState(page: Page) {
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
			if (stepId === 'devicename') {
				await browserPage.locator('#device-name').fill('test-device');
				await browserPage.locator('#hostname').fill('test-device');
			}
			if (stepId === 'whoareyou') {
				await browserPage.locator('#full-name').fill('Test User');
				await browserPage.locator('#username').fill('testuser');
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
	});
}
