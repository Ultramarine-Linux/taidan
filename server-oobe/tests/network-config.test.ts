// Address helpers, the network.configure payload and its validation, the
// network check, and safe apply in the OOBE, without a browser.
import { expect, test } from '@playwright/test';
import { isIP } from 'node:net';
import { addressScope, ipFamily, oobeUrl, urlHost } from '../src/lib/net';
import {
	emptyIpForm,
	expectedAddresses,
	interfaceType,
	ipSettings
} from '../src/lib/network-config';
import { validateNetworkConfig } from '../src/lib/server/network-config';
import { checkNetwork } from '../src/lib/server/network-check';
import { applyChange, confirmChange } from '../src/lib/server/network-change';
import type { TetraResponse } from '../src/lib/server/tetra';

test.describe('addresses', () => {
	test('ipFamily agrees with node:net', () => {
		for (const value of [
			'192.0.2.1',
			'0.0.0.0',
			'255.255.255.255',
			'256.1.1.1',
			'01.2.3.4',
			'1.2.3',
			'::',
			'::1',
			'2001:db8::1',
			'2001:db8:0:0:0:0:0:1',
			'2001:db8::1::2',
			'::ffff:192.0.2.1',
			'64:ff9b::192.0.2.33',
			'1:2:3:4:5:6:7:8',
			'1:2:3:4:5:6:7:8:9',
			'1:2:3:4:5:6:7::',
			'fe80::1',
			'gggg::1',
			':1::',
			'[2001:db8::1]',
			'example.com',
			''
		]) {
			expect(ipFamily(value), value).toBe(isIP(value));
		}
	});

	test('scopes follow Tetra', () => {
		expect(addressScope('203.0.113.5')).toBe('global');
		expect(addressScope('192.168.1.42')).toBe('private');
		expect(addressScope('100.64.0.1')).toBe('private');
		expect(addressScope('169.254.0.1')).toBe('link-local');
		expect(addressScope('127.0.0.1')).toBe('loopback');
		expect(addressScope('2001:db8::1')).toBe('global');
		expect(addressScope('fd00::1')).toBe('ula');
		expect(addressScope('fe80::1')).toBe('link-local');
		expect(addressScope('::1')).toBe('loopback');
		expect(addressScope('ff02::1')).toBe('multicast');
		expect(addressScope('::')).toBe('unspecified');
	});

	test('URLs put IPv6 in brackets', () => {
		expect(urlHost('192.0.2.1')).toBe('192.0.2.1');
		expect(oobeUrl('192.0.2.1')).toBe('https://192.0.2.1:6623');
		expect(oobeUrl('2001:db8::1')).toBe('https://[2001:db8::1]:6623');
	});
});

test('interface type comes from Tetra, with the wl prefix as a fallback', () => {
	expect(interfaceType({ name: 'wlp2s0', type: 'wifi' })).toBe('wifi');
	// Tetra's type wins over the name.
	expect(interfaceType({ name: 'wlan0', type: 'other' })).toBe('other');
	expect(interfaceType({ name: 'mywifi', type: 'wifi' })).toBe('wifi');
	expect(interfaceType({ name: 'br0', type: 'other' })).toBe('other');
	// Older Tetra reports no type.
	expect(interfaceType({ name: 'wlan0' })).toBe('wifi');
	expect(interfaceType({ name: 'enp1s0' })).toBe('ethernet');
	expect(interfaceType({ name: 'enp1s0', type: 'token-ring' })).toBe('ethernet');
});

test.describe('network.configure payload', () => {
	test('the form becomes Tetra sections', () => {
		expect(ipSettings(emptyIpForm('auto'))).toEqual({ method: 'auto' });
		expect(
			ipSettings({
				method: 'manual',
				addresses: '2001:db8::2/64, [2001:db8::3]/64 2001:db8::4',
				gateway: '[fe80::1]',
				dns: '2606:4700:4700::1111,2620:fe::fe'
			})
		).toEqual({
			method: 'manual',
			addresses: [
				{ address: '2001:db8::2', prefix_length: 64 },
				{ address: '2001:db8::3', prefix_length: 64 },
				// Sent without a prefix so the server can say what is missing.
				{ address: '2001:db8::4', prefix_length: 0 }
			],
			gateway: 'fe80::1',
			dns: ['2606:4700:4700::1111', '2620:fe::fe']
		});
	});

	test('expected addresses are the manual ones and the current automatic ones', () => {
		const current = [
			{ family: 'ipv4', address: '192.0.2.5', prefix_length: 24, scope: 'global' },
			{ family: 'ipv6', address: '2001:db8::5', prefix_length: 64, scope: 'global' },
			{ family: 'ipv6', address: 'fe80::5', prefix_length: 64, scope: 'link-local' }
		] as const;
		expect(
			expectedAddresses(
				{
					ipv4: { method: 'disabled' },
					ipv6: { method: 'manual', addresses: [{ address: '2001:db8::9', prefix_length: 64 }] }
				},
				[...current]
			)
		).toEqual({ addresses: ['2001:db8::9'], automatic: false });
		expect(expectedAddresses({ ipv6: { method: 'dhcp' } }, [...current])).toEqual({
			addresses: ['192.0.2.5', '2001:db8::5'],
			automatic: true
		});
	});
});

test.describe('validateNetworkConfig', () => {
	const manual6 = {
		method: 'manual',
		addresses: [{ address: '2001:db8:10::20', prefix_length: 64 }],
		gateway: 'fe80::1',
		dns: ['2606:4700:4700::1111']
	};

	test('accepts IPv6-only manual configuration', () => {
		expect(
			validateNetworkConfig({ interface: 'enp1s0', ipv4: { method: 'disabled' }, ipv6: manual6 })
		).toEqual({ interface: 'enp1s0', ipv4: { method: 'disabled' }, ipv6: manual6 });
	});

	test('no section is required', () => {
		expect(validateNetworkConfig({ interface: 'enp1s0' })).toEqual({ interface: 'enp1s0' });
		expect(validateNetworkConfig({ interface: 'enp1s0', ipv6: { method: 'dhcp' } })).toEqual({
			interface: 'enp1s0',
			ipv6: { method: 'dhcp' }
		});
	});

	test('rejects both families off', () => {
		expect(
			validateNetworkConfig({
				interface: 'enp1s0',
				ipv4: { method: 'disabled' },
				ipv6: { method: 'disabled' }
			})
		).toBe('IPv4 and IPv6 cannot both be off');
		expect(
			validateNetworkConfig({
				interface: 'enp1s0',
				ipv4: { method: 'disabled' },
				ipv6: { method: 'link-local' }
			})
		).toBe('IPv4 and IPv6 cannot both be off');
	});

	test('every address must match its family', () => {
		const check = (ipv6: Record<string, unknown>) =>
			validateNetworkConfig({ interface: 'enp1s0', ipv6 });
		expect(check({ ...manual6, addresses: [{ address: '192.0.2.1', prefix_length: 24 }] })).toBe(
			'Not an IPv6 address: 192.0.2.1'
		);
		expect(check({ ...manual6, gateway: '192.0.2.1' })).toBe(
			'The IPv6 gateway must be an IPv6 address'
		);
		expect(check({ ...manual6, gateway: 'fe80::1%enp1s0' })).toBe(
			'The IPv6 gateway must be an IPv6 address'
		);
		expect(check({ ...manual6, dns: ['1.1.1.1'] })).toBe('IPv6 DNS servers must be IPv6 addresses');
		expect(
			validateNetworkConfig({
				interface: 'enp1s0',
				ipv4: { method: 'manual', addresses: [{ address: '2001:db8::1', prefix_length: 64 }] }
			})
		).toBe('Not an IPv4 address: 2001:db8::1');
	});

	test('manual needs addresses with prefix lengths, and only manual takes them', () => {
		const check = (ipv6: Record<string, unknown>) =>
			validateNetworkConfig({ interface: 'enp1s0', ipv6 });
		expect(check({ method: 'manual' })).toBe('Enter at least one IPv6 address');
		expect(check({ ...manual6, addresses: [{ address: '2001:db8::1', prefix_length: 0 }] })).toBe(
			'Enter each IPv6 address with a prefix length from 1 to 128, such as 2001:db8::2/64'
		);
		expect(check({ method: 'auto', addresses: manual6.addresses })).toBe(
			'IPv6 addresses need manual configuration'
		);
		expect(check({ method: 'auto', gateway: 'fe80::1' })).toBe(
			'An IPv6 gateway needs manual configuration'
		);
		expect(check({ method: 'auto', dns: ['2606:4700:4700::1111'] })).toMatchObject({
			ipv6: { method: 'auto', dns: ['2606:4700:4700::1111'] }
		});
		expect(check({ method: 'disabled', dns: ['2606:4700:4700::1111'] })).toBe(
			'IPv6 DNS servers need IPv6 to be on'
		);
		expect(check({ method: 'static' })).toBe(
			'IPv6 method must be auto, dhcp, manual, link-local, disabled'
		);
	});

	test('refuses fields Tetra does not know', () => {
		expect(validateNetworkConfig({ interface: 'enp1s0', ipv4: { method: 'dhcp' } })).toBe(
			'IPv4 method must be auto, manual, disabled'
		);
		expect(validateNetworkConfig({ interface: 'enp1s0', mtu: 1500 })).toBe(
			'Unknown network setting: mtu'
		);
		expect(validateNetworkConfig({ interface: 'enp1s0', ipv6: { method: 'auto', mtu: 1 } })).toBe(
			'Unknown IPv6 setting: mtu'
		);
		expect(
			validateNetworkConfig({
				interface: 'enp1s0',
				wired_8021x: { eap: 'peap', identity: 'a', username: 'a' }
			})
		).toBe('Wired 802.1X: Unknown 802.1X setting: username');
	});
});

test.describe('network check', () => {
	const iface = (addresses: [string, string][]) => ({
		name: 'enp1s0',
		addresses: addresses.map(([family, address]) => ({ family, address }))
	});

	test('reports routable addresses per family and DNS', async () => {
		expect(
			await checkNetwork(
				[
					iface([
						['ipv6', '2001:db8::2'],
						['ipv6', 'fe80::1'],
						['ipv4', '127.0.0.1']
					])
				],
				async () => true
			)
		).toEqual({ ipv4: false, ipv6: true, dns: true });
		expect(
			await checkNetwork(
				[
					iface([
						['ipv4', '192.168.1.2'],
						['ipv6', 'fe80::1']
					])
				],
				async () => false
			)
		).toEqual({ ipv4: true, ipv6: false, dns: false });
		expect(await checkNetwork(undefined, async () => false)).toEqual({
			ipv4: false,
			ipv6: false,
			dns: false
		});
	});
});

test.describe('safe apply', () => {
	const ok = (payload: Record<string, unknown>): TetraResponse => ({ ok: true, payload });

	test('a change Tetra wants confirmed is confirmed once, with its token', async () => {
		const confirmed: string[][] = [];
		const confirm = async (session: string, token: string) => {
			confirmed.push([session, token]);
			return ok({ confirmed: true });
		};
		const id = 'unit-confirm-000000000001';
		await applyChange(id, 'tts_session', async () =>
			ok({ confirmation_required: true, confirmation_token: 'ct1', rollback_in_seconds: 60 })
		);
		const [first, second] = await Promise.all([
			confirmChange(id, confirm),
			confirmChange(id, confirm)
		]);
		expect(first).toEqual({ status: 'kept', message: undefined });
		expect(second).toEqual({ status: 'kept', message: undefined });
		expect(confirmed).toEqual([['tts_session', 'ct1']]);
	});

	test('confirm waits for a change still being applied', async () => {
		let finish: (resp: TetraResponse) => void = () => {};
		const id = 'unit-waits-00000000000001';
		void applyChange(id, 'tts_session', () => new Promise((resolve) => (finish = resolve)));
		const result = confirmChange(id, async () => ok({}));
		finish(ok({ confirmation_required: false }));
		expect(await result).toMatchObject({ status: 'kept' });
	});

	test('failed, expired and unknown changes', async () => {
		await applyChange('unit-failed-0000001000000', 't', async () => ({ ok: false, error: 'bad' }));
		expect(await confirmChange('unit-failed-0000001000000', async () => ok({}))).toEqual({
			status: 'failed',
			message: 'bad'
		});

		await applyChange('unit-expired-000001000000', 't', async () =>
			ok({ confirmation_required: true, confirmation_token: 'ct', rollback_in_seconds: 0 })
		);
		expect(await confirmChange('unit-expired-000001000000', async () => ok({}))).toMatchObject({
			status: 'rolled-back'
		});

		// A refused confirm (Tetra unreachable) can be retried.
		await applyChange('unit-retry-00000001000000', 't', async () =>
			ok({ confirmation_required: true, confirmation_token: 'ct', rollback_in_seconds: 60 })
		);
		expect(
			await confirmChange('unit-retry-00000001000000', async () => ({ ok: false, error: 'down' }))
		).toEqual({ status: 'unknown', message: 'down' });
		expect(await confirmChange('unit-retry-00000001000000', async () => ok({}))).toMatchObject({
			status: 'kept'
		});

		expect(await confirmChange('unit-never-00000001000000', async () => ok({}))).toEqual({
			status: 'unknown'
		});
	});

	test('an id names one change and keeps it at most once', async () => {
		let runs = 0;
		let confirms = 0;
		const run = async () => {
			runs += 1;
			return ok({ confirmation_required: true, confirmation_token: 'ct', rollback_in_seconds: 60 });
		};
		const id = 'unit-once-000000000000001';
		await applyChange(id, 'tts_a', run);
		// Reusing the id cannot bind it to another change.
		expect(await applyChange(id, 'tts_b', run)).toEqual({
			ok: false,
			error: 'This change id was already used'
		});
		expect(runs).toBe(1);
		const keep = async () => {
			confirms += 1;
			return ok({ confirmed: true });
		};
		expect(await confirmChange(id, keep)).toMatchObject({ status: 'kept' });
		expect(await confirmChange(id, keep)).toMatchObject({ status: 'kept' });
		expect(confirms).toBe(1);
	});

	test('a settled outcome is dropped after a while', async () => {
		const id = 'unit-forget-0000000000001';
		await applyChange(id, 't', async () => ok({ confirmation_required: false }), 50);
		expect(await confirmChange(id, async () => ok({}))).toMatchObject({ status: 'kept' });
		await new Promise((resolve) => setTimeout(resolve, 100));
		expect(await confirmChange(id, async () => ok({}))).toEqual({ status: 'unknown' });
	});
});
