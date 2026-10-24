import { expect, localURL, test } from './accessibility';
import { FIXTURE_HOST_COOKIE } from '../src/lib/fixture-hosts';
import { FIXTURE_SETUP_CODE } from '../src/lib/fixture';

test('network endpoint returns routable IPv4 and IPv6 addresses, IPv4 first', async ({
	request
}) => {
	// The dual-stack fixture host also has loopback and link-local addresses.
	const res = await request.get(`${localURL}/api/oobe/network`);
	expect(res.ok()).toBe(true);
	const { addresses } = (await res.json()) as { addresses: string[] };
	expect(addresses).toEqual(['192.168.1.42', '2001:db8:1::42', 'fd00:1::42']);
});

test('network endpoint on an IPv6-only host', async ({ request }) => {
	const res = await request.get(`${localURL}/api/oobe/network`, {
		headers: { cookie: `${FIXTURE_HOST_COOKIE}=ipv6-only` }
	});
	expect(await res.json()).toEqual({ addresses: ['2001:db8:10::2'] });
});

test('welcome step shows URLs on port 6623, IPv6 in brackets', async ({ page }) => {
	await page.route('**/api/oobe/network', (route) =>
		route.fulfill({ json: { addresses: ['192.0.2.10', '2001:db8::1'] } })
	);
	await page.goto(localURL);

	const ipAddresses = page.locator('.ip-addresses');
	await expect(ipAddresses).toContainText('Configure this server from another device at:');
	await expect(ipAddresses.locator('li')).toHaveText([
		'https://192.0.2.10:6623',
		'https://[2001:db8::1]:6623'
	]);
});

test('welcome step on an IPv6-only fixture host', async ({ page }) => {
	await page
		.context()
		.addCookies([{ name: FIXTURE_HOST_COOKIE, value: 'ipv6-only', url: localURL }]);
	await page.goto(localURL);
	await expect(page.locator('.ip-addresses li')).toHaveText(['https://[2001:db8:10::2]:6623']);
});

test('welcome step hides IP addresses when none are available', async ({ page }) => {
	await page.route('**/api/oobe/network', (route) => route.fulfill({ json: { addresses: [] } }));
	await page.goto(localURL);

	await expect(page.locator('.workspace h2')).toBeVisible();
	await expect(page.locator('.ip-addresses')).toHaveCount(0);
});

test('a link to the new address keeps a network change without a session', async ({
	page,
	request,
	playwright
}) => {
	// Change the IPv6-only host's address; Tetra then waits for network.confirm.
	await request.post(`${localURL}/api/oobe/session`, {
		data: { code: FIXTURE_SETUP_CODE },
		headers: { cookie: `${FIXTURE_HOST_COOKIE}=ipv6-only` }
	});
	const changeId = `link-${Date.now()}-abcdefgh`;
	const res = await request.post(`${localURL}/api/oobe/operations`, {
		data: {
			step: 'internet',
			operation: 'network.configure',
			payload: {
				changeId,
				interface: 'enp1s0',
				ipv4: { method: 'disabled' },
				ipv6: { method: 'manual', addresses: [{ address: '2001:db8:10::30', prefix_length: 64 }] }
			}
		}
	});
	expect(await res.json()).toMatchObject({
		status: 'succeeded',
		message: JSON.stringify({ confirmationRequired: true, rollbackInSeconds: 60 })
	});

	// A browser without the session cookie opens the link shown for the new address.
	await page.goto(`${localURL}/?network-change=${changeId}`);
	await expect(page.getByRole('status')).toContainText('The new network settings are kept.');
	await expect(page).toHaveURL(`${localURL}/`);

	const anonymous = await playwright.request.newContext();
	const again = await anonymous.post(`${localURL}/api/oobe/network/confirm`, {
		data: { changeId }
	});
	expect(await again.json()).toEqual({ status: 'kept' });
	const unknown = await anonymous.post(`${localURL}/api/oobe/network/confirm`, {
		data: { changeId: 'never-made-this-change' }
	});
	expect(await unknown.json()).toEqual({ status: 'unknown' });
	expect(
		(
			await anonymous.post(`${localURL}/api/oobe/network/confirm`, { data: { changeId: 'x' } })
		).status()
	).toBe(400);
	await anonymous.dispose();
});
