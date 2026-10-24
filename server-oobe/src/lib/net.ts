// IP address helpers shared by the browser and the server: address families,
// Tetra's address scopes, and OOBE URLs with IPv6 in brackets.
//
// This module must not import SvelteKit aliases ($lib, $app, $env) so that
// tests and the Tetra mock can load it directly.

/** Port browsers use to reach the OOBE (Caddy, run by Tetra, on IPv4 and IPv6). */
export const OOBE_PORT = 6623;

/**
 * Tetra's address scopes (docs/agent-protocol.md, `network.interfaces`),
 * derived from the address prefix. `global`, `private` and `ula` are routable.
 */
export type AddressScope =
	'global' | 'private' | 'ula' | 'link-local' | 'loopback' | 'multicast' | 'unspecified';

const IPV4_PATTERN = /^(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3}$/;

function ipv4Octets(value: string): number[] | undefined {
	return IPV4_PATTERN.test(value) ? value.split('.').map(Number) : undefined;
}

/** The eight 16-bit groups of an IPv6 address, or undefined if `value` is not one. */
function ipv6Groups(value: string): number[] | undefined {
	let text = value;
	const tail: number[] = [];
	if (text.includes('.')) {
		// An embedded IPv4 address, as in ::ffff:192.0.2.1, fills the last two groups.
		const lastColon = text.lastIndexOf(':');
		const octets = lastColon === -1 ? undefined : ipv4Octets(text.slice(lastColon + 1));
		if (!octets) return undefined;
		tail.push((octets[0] << 8) | octets[1], (octets[2] << 8) | octets[3]);
		text = text.slice(0, lastColon + 1);
		if (!text.endsWith('::')) text = text.slice(0, -1);
	}
	const halves = text.split('::');
	if (halves.length > 2) return undefined;
	const parse = (part: string) => (part === '' ? [] : part.split(':'));
	const head = parse(halves[0]);
	const rest = halves.length === 2 ? parse(halves[1]) : [];
	if (![...head, ...rest].every((group) => /^[0-9a-f]{1,4}$/i.test(group))) return undefined;
	const known = head.length + rest.length + tail.length;
	if (halves.length === 1 ? known !== 8 : known > 7) return undefined;
	const numbers = (groups: string[]) => groups.map((group) => parseInt(group, 16));
	return [...numbers(head), ...new Array<number>(8 - known).fill(0), ...numbers(rest), ...tail];
}

/** 4 or 6 for a valid IPv4 or IPv6 address, otherwise 0 (like `node:net`'s `isIP`). */
export function ipFamily(value: string): 0 | 4 | 6 {
	if (ipv4Octets(value)) return 4;
	return ipv6Groups(value) ? 6 : 0;
}

/** Tetra's scope for an address, or undefined if it is not an IP address. */
export function addressScope(address: string): AddressScope | undefined {
	const octets = ipv4Octets(address);
	if (octets) {
		const [a, b] = octets;
		if (a === 0) return 'unspecified';
		if (a === 127) return 'loopback';
		if (a === 169 && b === 254) return 'link-local';
		if (a >= 224 && a <= 239) return 'multicast';
		if (a === 10 || (a === 172 && b >= 16 && b <= 31) || (a === 192 && b === 168)) {
			return 'private';
		}
		if (a === 100 && b >= 64 && b <= 127) return 'private';
		return 'global';
	}
	const groups = ipv6Groups(address);
	if (!groups) return undefined;
	if (groups.every((group) => group === 0)) return 'unspecified';
	if (groups.slice(0, 7).every((group) => group === 0) && groups[7] === 1) return 'loopback';
	if ((groups[0] & 0xffc0) === 0xfe80) return 'link-local';
	if ((groups[0] & 0xff00) === 0xff00) return 'multicast';
	if ((groups[0] & 0xfe00) === 0xfc00) return 'ula';
	return 'global';
}

/** Whether Tetra counts an address of this scope as giving the host an address others can reach. */
export function isRoutable(scope: AddressScope | undefined): boolean {
	return scope === 'global' || scope === 'private' || scope === 'ula';
}

/** `address` as the host part of a URL: IPv6 in brackets. */
export function urlHost(address: string): string {
	return address.includes(':') ? `[${address}]` : address;
}

/** Remove the brackets around an IPv6 address, if any. */
export function stripBrackets(value: string): string {
	return value.replace(/^\[(.*)\]$/, '$1');
}

/** Where browsers reach the OOBE on `address`, e.g. `https://[2001:db8::1]:6623`. */
export function oobeUrl(address: string, port: number = OOBE_PORT): string {
	return `https://${urlHost(address)}:${port}`;
}
