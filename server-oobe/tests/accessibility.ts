import AxeBuilder from '@axe-core/playwright';
import { test as base } from '@playwright/test';

export const localURL = 'http://127.0.0.1:4173';

export const test = base.extend<{ makeAxeBuilder: () => AxeBuilder }>({
	makeAxeBuilder: async ({ page }, use) => {
		const makeAxeBuilder = () =>
			new AxeBuilder({ page }).withTags([
				'wcag2a',
				'wcag21a',
				'wcag2aa',
				'wcag21aa',
				'wcag22aa',
				'best-practice'
			]);
		await use(makeAxeBuilder);
	}
});

export { expect } from '@playwright/test';
