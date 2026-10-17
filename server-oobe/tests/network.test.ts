import { expect, localURL, test } from './accessibility';

test('network endpoint returns only non-internal IPv4 addresses', async ({ request }) => {
	const res = await request.get(`${localURL}/api/oobe/network`);
	expect(res.ok()).toBe(true);

	const { addresses } = (await res.json()) as { addresses: string[] };
	expect(Array.isArray(addresses)).toBe(true);
	expect(new Set(addresses).size).toBe(addresses.length);
	for (const address of addresses) {
		expect(address).toMatch(/^\d{1,3}(\.\d{1,3}){3}$/);
		expect(address.startsWith('127.')).toBe(false);
	}
});

test('welcome step shows server IP addresses', async ({ page }) => {
	await page.route('**/api/oobe/network', (route) =>
		route.fulfill({ json: { addresses: ['192.0.2.10', '198.51.100.7'] } })
	);
	await page.goto(localURL);

	const ipAddresses = page.locator('.ip-addresses');
	await expect(ipAddresses).toContainText('Configure this server from another device at:');
	await expect(ipAddresses).toContainText('192.0.2.10, 198.51.100.7');
});

test('welcome step hides IP addresses when none are available', async ({ page }) => {
	await page.route('**/api/oobe/network', (route) => route.fulfill({ json: { addresses: [] } }));
	await page.goto(localURL);

	await expect(page.locator('.workspace h2')).toBeVisible();
	await expect(page.locator('.ip-addresses')).toHaveCount(0);
});
