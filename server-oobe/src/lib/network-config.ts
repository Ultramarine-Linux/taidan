// Tetra's `network.configure` request (docs/agent-protocol.md, network
// section) and the internet step's form for it. Tetra rejects unknown fields,
// so only these names are sent.
//
// This module must not import SvelteKit aliases ($lib, $app, $env) so that
// tests can load it directly.

import type { Eap8021x } from './eap-8021x';
import { isRoutable, stripBrackets, type AddressScope } from './net';

export const IPV4_METHODS = ['auto', 'manual', 'disabled'] as const;
export const IPV6_METHODS = ['auto', 'dhcp', 'manual', 'link-local', 'disabled'] as const;

/** `auto` is DHCP. */
export type Ipv4Method = (typeof IPV4_METHODS)[number];
/** `auto` is SLAAC (with DHCPv6 when router advertisements ask for it); `dhcp` is DHCPv6 only. */
export type Ipv6Method = (typeof IPV6_METHODS)[number];

export type IpAddressConfig = { address: string; prefix_length: number };

/** One family's section. `addresses` and `gateway` only with `manual`. */
export type IpConfig<M> = {
	method: M;
	addresses?: IpAddressConfig[];
	gateway?: string;
	dns?: string[];
};

export type NetworkConfigureRequest = {
	interface: string;
	ipv4?: IpConfig<Ipv4Method>;
	ipv6?: IpConfig<Ipv6Method>;
	wired_8021x?: Eap8021x;
};

/**
 * `type` from Tetra's `network.interfaces`: `ethernet` is physical Ethernet
 * only; bridges, VLANs, tunnels and other virtual interfaces are `other`.
 */
export const INTERFACE_TYPES = ['wifi', 'ethernet', 'other'] as const;
export type InterfaceType = (typeof INTERFACE_TYPES)[number];

/**
 * An interface's type, from Tetra when it reports one. Older Tetra does not;
 * then wireless interfaces are recognised by their `wl` name prefix and the
 * rest are taken to be Ethernet.
 */
export function interfaceType(entry: { name: string; type?: unknown }): InterfaceType {
	if (INTERFACE_TYPES.includes(entry.type as InterfaceType)) return entry.type as InterfaceType;
	return entry.name.startsWith('wl') ? 'wifi' : 'ethernet';
}

/** One address from Tetra's `network.interfaces`. */
export type InterfaceAddress = {
	family: 'ipv4' | 'ipv6';
	address: string;
	prefix_length: number;
	scope: AddressScope;
};

/** What a family's form fields hold. Lists are comma- or space-separated. */
export type IpForm<M> = {
	method: M;
	/** Addresses with prefix lengths, e.g. `2001:db8::2/64`. */
	addresses: string;
	gateway: string;
	dns: string;
};

export function emptyIpForm<M>(method: M): IpForm<M> {
	return { method, addresses: '', gateway: '', dns: '' };
}

function list(value: string): string[] {
	return value
		.split(/[\s,]+/)
		.map((item) => item.trim())
		.filter(Boolean);
}

/** Build a family's section from the form. The server checks it; this only reshapes it. */
export function ipSettings<M extends string>(form: IpForm<M>): IpConfig<M> {
	if (form.method !== 'manual') return { method: form.method };
	const gateway = stripBrackets(form.gateway.trim());
	const dns = list(form.dns).map(stripBrackets);
	return {
		method: form.method,
		addresses: list(form.addresses).map((entry) => {
			const slash = entry.lastIndexOf('/');
			const address = stripBrackets(slash === -1 ? entry : entry.slice(0, slash));
			// A missing prefix length is sent as 0 so the server can say so.
			const prefix = slash === -1 ? 0 : Number(entry.slice(slash + 1));
			return { address, prefix_length: Number.isInteger(prefix) ? prefix : 0 };
		}),
		...(gateway ? { gateway } : {}),
		...(dns.length ? { dns } : {})
	};
}

/**
 * The addresses the interface should have after `config` is applied: the
 * manual ones, and for automatic families the routable ones it has now (DHCP
 * and SLAAC usually hand them out again). Used to tell the user where to
 * find the OOBE if the browser loses its connection.
 */
export function expectedAddresses(
	config: Pick<NetworkConfigureRequest, 'ipv4' | 'ipv6'>,
	current: InterfaceAddress[] = []
): { addresses: string[]; automatic: boolean } {
	const addresses: string[] = [];
	let automatic = false;
	for (const family of ['ipv4', 'ipv6'] as const) {
		const method = config[family]?.method ?? 'auto';
		if (method === 'manual') {
			addresses.push(...(config[family]?.addresses ?? []).map((entry) => entry.address));
		} else if (method === 'auto' || method === 'dhcp') {
			automatic = true;
			addresses.push(
				...current
					.filter((entry) => entry.family === family && isRoutable(entry.scope))
					.map((entry) => entry.address)
			);
		}
	}
	return { addresses: [...new Set(addresses)], automatic };
}
