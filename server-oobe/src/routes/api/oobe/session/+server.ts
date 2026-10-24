import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { tetraClient, TetraError } from '$lib/server/tetra';
import { MockTetraClient } from '$lib/server/tetra-mock';
import { FIXTURE_HOST_COOKIE } from '$lib/fixture-hosts';
import { checkSetupSession, clearSessionCookie, setSessionCookie } from '$lib/server/setup-session';
import type { SessionResponse } from '$lib/oobe-state';

function unavailable(e: unknown) {
	return json(
		{
			authenticated: false,
			setupRequired: true,
			error: 'unavailable',
			message: e instanceof Error ? e.message : String(e)
		} satisfies SessionResponse,
		{ status: 503 }
	);
}

/** Whether this browser already holds a live setup session. */
export const GET: RequestHandler = async ({ cookies }) => {
	const session = await checkSetupSession(cookies);
	if (session.status === 'unavailable') return unavailable(new Error(session.message));
	if (session.status === 'valid') {
		return json({ authenticated: true, setupRequired: true } satisfies SessionResponse);
	}
	try {
		const { setupRequired } = await tetraClient().setupStatus();
		return json({ authenticated: false, setupRequired } satisfies SessionResponse);
	} catch (e) {
		return unavailable(e);
	}
};

/** Redeem the setup code from the server console for a setup session. */
export const POST: RequestHandler = async ({ request, cookies, url, getClientAddress }) => {
	let code = '';
	try {
		const body = (await request.json()) as { code?: unknown };
		code = typeof body.code === 'string' ? body.code.trim() : '';
	} catch {
		// Treated as an empty code below.
	}
	if (!code || code.length > 64) {
		return json(
			{
				authenticated: false,
				setupRequired: true,
				error: 'invalid_code'
			} satisfies SessionResponse,
			{ status: 400 }
		);
	}
	try {
		// With the unit's ADDRESS_HEADER/XFF_DEPTH this is the browser's address as
		// Caddy saw it, not Caddy's own.
		const client = tetraClient();
		const session = await client.redeemSetupCode(code, getClientAddress());
		// Fixture mode: the session sees the host the tests picked.
		if (client instanceof MockTetraClient) {
			client.useFixtureHost(session.token, cookies.get(FIXTURE_HOST_COOKIE));
		}
		setSessionCookie(cookies, session, url.protocol === 'https:');
		return json({
			authenticated: true,
			setupRequired: true,
			adminExists: session.adminExists
		} satisfies SessionResponse);
	} catch (e) {
		clearSessionCookie(cookies);
		if (!(e instanceof TetraError)) throw e;
		switch (e.code) {
			case 'invalid_credentials':
				return json(
					{
						authenticated: false,
						setupRequired: true,
						error: 'invalid_code'
					} satisfies SessionResponse,
					{ status: 401 }
				);
			case 'rate_limited':
				return json(
					{
						authenticated: false,
						setupRequired: true,
						error: 'rate_limited',
						retryAfter: e.retryAfter
					} satisfies SessionResponse,
					{ status: 429, headers: e.retryAfter ? { 'Retry-After': String(e.retryAfter) } : {} }
				);
			case 'setup_complete':
				return json(
					{
						authenticated: false,
						setupRequired: false,
						error: 'setup_complete'
					} satisfies SessionResponse,
					{ status: 409 }
				);
			default:
				return unavailable(e);
		}
	}
};
