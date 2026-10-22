import { json } from '@sveltejs/kit';
import { networkInterfaces } from 'node:os';
import { FIXTURE_HOST_COOKIE, fixtureHost } from '$lib/fixture-hosts';
import { addressScope, isRoutable } from '$lib/net';
import type { RequestHandler } from './$types';

const FIXTURE_MODE = process.env.OOBE_FIXTURE_MODE === 'true';

// The host's routable addresses (global and private IPv4, global and unique
// local IPv6; no loopback or link-local), so the welcome screen can tell users
// where to reach this OOBE from another device. IPv4 first. Node runs on the
// host, so it sees the host's addresses; fixture mode uses the fixture host.
export const GET: RequestHandler = ({ cookies }) => {
	const all = FIXTURE_MODE
		? fixtureHost(cookies.get(FIXTURE_HOST_COOKIE)).interfaces.flatMap((entry) =>
				entry.addresses.map((address) => address.address)
			)
		: Object.values(networkInterfaces())
				.flatMap((entries) => entries ?? [])
				.filter((entry) => !entry.internal)
				.map((entry) => entry.address);
	const routable = all.filter((address) => isRoutable(addressScope(address)));
	const addresses = [
		...routable.filter((address) => !address.includes(':')),
		...routable.filter((address) => address.includes(':'))
	];

	return json({ addresses: [...new Set(addresses)] });
};
