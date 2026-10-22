// `network.check`: whether the host has a routable address per family (from
// Tetra's `network.interfaces`) and whether DNS resolves.
//
// This module must not import SvelteKit aliases ($lib, $app, $env) so that
// tests can load it directly.

import { lookup } from 'node:dns/promises';
import { addressScope, isRoutable } from '../net';

/** Name resolved to see whether DNS works. */
export const DNS_CHECK_HOST = process.env.OOBE_DNS_CHECK_HOST || 'ultramarine-linux.org';
const DNS_TIMEOUT_MS = 5000;

export type NetworkCheck = {
	/** The host has a global or private IPv4 address. */
	ipv4: boolean;
	/** The host has a global or unique local IPv6 address. */
	ipv6: boolean;
	/** `DNS_CHECK_HOST` resolves. */
	dns: boolean;
};

/** Resolve `DNS_CHECK_HOST` with the system resolver; false on failure or timeout. */
export async function dnsResolves(host: string = DNS_CHECK_HOST): Promise<boolean> {
	let timer: ReturnType<typeof setTimeout> | undefined;
	const timeout = new Promise<boolean>((resolve) => {
		timer = setTimeout(() => resolve(false), DNS_TIMEOUT_MS);
	});
	try {
		return await Promise.race([
			lookup(host, { all: true }).then((addresses) => addresses.length > 0),
			timeout
		]);
	} catch {
		return false;
	} finally {
		clearTimeout(timer);
	}
}

/** Summarize Tetra's `network.interfaces` result and a DNS probe. */
export async function checkNetwork(
	interfaces: unknown,
	resolves: () => Promise<boolean> = dnsResolves
): Promise<NetworkCheck> {
	const addresses = (Array.isArray(interfaces) ? interfaces : []).flatMap((entry) =>
		Array.isArray(entry?.addresses) ? (entry.addresses as unknown[]) : []
	);
	const routable = (family: 'ipv4' | 'ipv6') =>
		addresses.some((entry) => {
			const { family: entryFamily, address } = (entry ?? {}) as Record<string, unknown>;
			// Tetra classifies by prefix; classify again rather than trust a missing scope.
			return (
				entryFamily === family && typeof address === 'string' && isRoutable(addressScope(address))
			);
		});
	return { ipv4: routable('ipv4'), ipv6: routable('ipv6'), dns: await resolves() };
}
