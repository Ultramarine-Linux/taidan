// Hosts the mock Tetra pretends to be in fixture mode (development and tests).
// The `oobe_fixture_host` cookie picks one when the setup code is redeemed
// and for /api/oobe/network; without it the host is dual-stack.
//
// This module must not import SvelteKit aliases ($lib, $app, $env) so that
// tests and the Tetra mock can load it directly.

import { addressScope, type AddressScope } from './net';
import type { InterfaceType } from './network-config';

export const FIXTURE_HOST_COOKIE = 'oobe_fixture_host';

export const FIXTURE_HOSTS = ['dual-stack', 'ipv6-only', 'wifi'] as const;
export type FixtureHostName = (typeof FIXTURE_HOSTS)[number];

/** An interface as Tetra's `network.interfaces` reports it. */
export type FixtureInterface = {
	name: string;
	type: InterfaceType;
	operstate: string;
	mac: string;
	addresses: {
		interface: string;
		family: 'ipv4' | 'ipv6';
		address: string;
		prefix_length: number;
		scope: AddressScope;
	}[];
};

/** A network as Tetra's `network.scan_wifi` reports it. */
export type FixtureWifiNetwork = {
	ssid: string;
	security: 'open' | 'wpa-personal' | 'wpa-enterprise' | 'unknown';
	signal: number;
	frequency: number;
	in_use: boolean;
};

export type FixtureHost = {
	interfaces: FixtureInterface[];
	wifiNetworks: FixtureWifiNetwork[];
};

export function fixtureInterface(
	name: string,
	type: InterfaceType,
	operstate: string,
	mac: string,
	cidrs: string[]
): FixtureInterface {
	return {
		name,
		type,
		operstate,
		mac,
		addresses: cidrs.map((cidr) => {
			const [address, prefix] = cidr.split('/');
			return {
				interface: name,
				family: address.includes(':') ? 'ipv6' : 'ipv4',
				address,
				prefix_length: Number(prefix),
				scope: addressScope(address) ?? 'global'
			};
		})
	};
}

const loopback = () =>
	fixtureInterface('lo', 'other', 'unknown', '00:00:00:00:00:00', ['127.0.0.1/8', '::1/128']);

const wifiNetworks = (connected?: string): FixtureWifiNetwork[] => [
	{
		ssid: 'Ultramarine-Setup',
		security: 'wpa-personal',
		signal: 86,
		frequency: 5180,
		in_use: connected === 'Ultramarine-Setup'
	},
	{ ssid: 'Corp Wi-Fi', security: 'wpa-enterprise', signal: 72, frequency: 2437, in_use: false },
	{ ssid: 'Guest Network', security: 'open', signal: 58, frequency: 2412, in_use: false }
];

const HOSTS: Record<FixtureHostName, () => FixtureHost> = {
	// Ethernet with DHCP, SLAAC and a ULA, an unused Wi-Fi card, and a bridge
	// with no address (a virtual interface setup does not configure).
	'dual-stack': () => ({
		interfaces: [
			fixtureInterface('enp1s0', 'ethernet', 'up', '52:54:00:12:34:56', [
				'192.168.1.42/24',
				'2001:db8:1::42/64',
				'fd00:1::42/64',
				'fe80::5054:ff:fe12:3456/64'
			]),
			loopback(),
			fixtureInterface('virbr0', 'other', 'down', '52:54:00:00:00:01', []),
			fixtureInterface('wlan0', 'wifi', 'down', '52:54:00:ab:cd:ef', [])
		],
		wifiNetworks: wifiNetworks()
	}),
	// A VPS with only a static IPv6 address.
	'ipv6-only': () => ({
		interfaces: [
			fixtureInterface('enp1s0', 'ethernet', 'up', '52:54:00:12:34:57', [
				'2001:db8:10::2/64',
				'fe80::5054:ff:fe12:3457/64'
			]),
			loopback()
		],
		wifiNetworks: []
	}),
	// A small box on Wi-Fi only.
	wifi: () => ({
		interfaces: [
			loopback(),
			fixtureInterface('wlp2s0', 'wifi', 'up', '52:54:00:ab:cd:f0', [
				'192.168.50.20/24',
				'2001:db8:50::20/64',
				'fe80::5054:ff:feab:cdf0/64'
			])
		],
		wifiNetworks: wifiNetworks('Ultramarine-Setup')
	})
};

export function isFixtureHostName(name: unknown): name is FixtureHostName {
	return FIXTURE_HOSTS.includes(name as FixtureHostName);
}

/** A fresh copy of a fixture host; unknown names get the dual-stack host. */
export function fixtureHost(name?: string): FixtureHost {
	return HOSTS[isFixtureHostName(name) ? name : 'dual-stack']();
}
