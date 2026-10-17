import { json } from '@sveltejs/kit';
import { networkInterfaces } from 'node:os';
import type { RequestHandler } from './$types';

// Non-internal IPv4 addresses, so the welcome screen can tell users where to
// reach this OOBE from another device.
export const GET: RequestHandler = () => {
	const addresses = Object.values(networkInterfaces())
		.flatMap((interfaces) => interfaces ?? [])
		.filter((address) => address.family === 'IPv4' && !address.internal)
		.map((address) => address.address);

	return json({ addresses: [...new Set(addresses)] });
};
