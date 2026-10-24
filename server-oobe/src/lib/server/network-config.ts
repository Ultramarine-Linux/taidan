// Checks a wired configuration from the browser and rebuilds it as Tetra's
// `network.configure` request, with only the fields Tetra knows.
//
// This module must not import SvelteKit aliases ($lib, $app, $env) so that
// tests can load it directly.

import { isIP } from 'node:net';
import { validateEap8021x } from '../eap-8021x';
import {
	IPV4_METHODS,
	IPV6_METHODS,
	type IpAddressConfig,
	type IpConfig,
	type NetworkConfigureRequest
} from '../network-config';

const INTERFACE_PATTERN = /^[a-zA-Z0-9_.-]{1,15}$/;
const REQUEST_FIELDS = ['interface', 'ipv4', 'ipv6', 'wired_8021x'];
const SECTION_FIELDS = ['method', 'addresses', 'gateway', 'dns'];

function isObject(value: unknown): value is Record<string, unknown> {
	return Boolean(value) && typeof value === 'object' && !Array.isArray(value);
}

/** Tetra parses addresses with Rust's `Ipv6Addr`, which has no zone index (`%eth0`). */
function isFamily(value: unknown, family: 4 | 6): value is string {
	return typeof value === 'string' && isIP(value) === family && !value.includes('%');
}

function ipSection<M extends string>(
	value: unknown,
	family: 4 | 6,
	methods: readonly M[]
): IpConfig<M> | string {
	const label = `IPv${family}`;
	if (!isObject(value)) return `${label} settings are invalid`;
	for (const key of Object.keys(value)) {
		if (!SECTION_FIELDS.includes(key)) return `Unknown ${label} setting: ${key}`;
	}
	const method = (value.method ?? 'auto') as M;
	if (!methods.includes(method)) return `${label} method must be ${methods.join(', ')}`;
	const manual = method === 'manual';
	const section: IpConfig<M> = { method };

	const addresses = value.addresses ?? [];
	if (!Array.isArray(addresses)) return `${label} addresses are invalid`;
	if (manual && addresses.length === 0) return `Enter at least one ${label} address`;
	if (!manual && addresses.length > 0) return `${label} addresses need manual configuration`;
	const maxPrefix = family === 4 ? 32 : 128;
	const cleaned: IpAddressConfig[] = [];
	for (const entry of addresses) {
		if (
			!isObject(entry) ||
			Object.keys(entry).some((key) => !['address', 'prefix_length'].includes(key))
		)
			return `${label} addresses are invalid`;
		if (!isFamily(entry.address, family))
			return `Not an ${label} address: ${String(entry.address)}`;
		const prefix = entry.prefix_length;
		if (typeof prefix !== 'number' || !Number.isInteger(prefix) || prefix < 1 || prefix > maxPrefix)
			return `Enter each ${label} address with a prefix length from 1 to ${maxPrefix}, such as ${family === 4 ? '192.0.2.10/24' : '2001:db8::2/64'}`;
		cleaned.push({ address: entry.address, prefix_length: prefix });
	}
	if (cleaned.length) section.addresses = cleaned;

	if (value.gateway !== undefined) {
		if (!manual) return `An ${label} gateway needs manual configuration`;
		// An IPv6 gateway may be link-local.
		if (!isFamily(value.gateway, family)) return `The ${label} gateway must be an ${label} address`;
		section.gateway = value.gateway;
	}

	const dns = value.dns ?? [];
	if (!Array.isArray(dns)) return `${label} DNS servers are invalid`;
	if (dns.length > 0 && (method === 'disabled' || method === 'link-local'))
		return `${label} DNS servers need ${label} to be on`;
	if (!dns.every((server) => isFamily(server, family)))
		return `${label} DNS servers must be ${label} addresses`;
	if (dns.length) section.dns = dns as string[];
	return section;
}

/**
 * Validate a wired configuration into Tetra's `network.configure` request, or
 * return an error message. Both sections are optional (Tetra defaults each to
 * `auto`), but at least one family must give the host a routable address.
 */
export function validateNetworkConfig(payload: unknown): NetworkConfigureRequest | string {
	if (!isObject(payload)) return 'Network settings are required';
	for (const key of Object.keys(payload)) {
		if (!REQUEST_FIELDS.includes(key)) return `Unknown network setting: ${key}`;
	}
	const iface = payload.interface;
	if (typeof iface !== 'string' || !INTERFACE_PATTERN.test(iface))
		return 'Invalid network interface';
	const request: NetworkConfigureRequest = { interface: iface };

	if (payload.ipv4 !== undefined) {
		const ipv4 = ipSection(payload.ipv4, 4, IPV4_METHODS);
		if (typeof ipv4 === 'string') return ipv4;
		request.ipv4 = ipv4;
	}
	if (payload.ipv6 !== undefined) {
		const ipv6 = ipSection(payload.ipv6, 6, IPV6_METHODS);
		if (typeof ipv6 === 'string') return ipv6;
		request.ipv6 = ipv6;
	}
	const ipv4Off = request.ipv4?.method === 'disabled';
	const ipv6Off = request.ipv6?.method === 'disabled' || request.ipv6?.method === 'link-local';
	if (ipv4Off && ipv6Off) return 'IPv4 and IPv6 cannot both be off';

	if (payload.wired_8021x !== undefined) {
		const wired = validateEap8021x(payload.wired_8021x);
		if (typeof wired === 'string') return `Wired 802.1X: ${wired}`;
		request.wired_8021x = wired;
	}
	return request;
}
