import type { Cookies } from '@sveltejs/kit';
import { tetraClient, TetraError, type SetupSession } from './tetra';

/**
 * HttpOnly cookie holding the Tetra setup session token. Browser JavaScript
 * never sees the token.
 */
export const SETUP_SESSION_COOKIE = 'oobe_setup_session';

export type SessionCheck =
	| { status: 'valid'; token: string }
	| { status: 'missing' }
	| { status: 'unavailable'; message: string };

/**
 * Ask Tetra whether the request's token is a live setup session for the OOBE.
 * Tetra decides; nothing is cached between requests.
 */
export async function checkSetupSession(cookies: Cookies): Promise<SessionCheck> {
	const token = cookies.get(SETUP_SESSION_COOKIE);
	if (!token) return { status: 'missing' };
	try {
		const info = await tetraClient().sessionInfo(token);
		if (info.kind === 'setup' && info.client === 'oobe') return { status: 'valid', token };
		return { status: 'missing' };
	} catch (e) {
		if (e instanceof TetraError && ['unauthenticated', 'wrong_client'].includes(e.code)) {
			return { status: 'missing' };
		}
		return { status: 'unavailable', message: e instanceof Error ? e.message : String(e) };
	}
}

export function setSessionCookie(cookies: Cookies, session: SetupSession, secure: boolean) {
	const maxAge = Math.max(0, session.expiresAt - Math.floor(Date.now() / 1000));
	cookies.set(SETUP_SESSION_COOKIE, session.token, {
		path: '/',
		httpOnly: true,
		sameSite: 'strict',
		secure,
		...(maxAge > 0 ? { maxAge } : {})
	});
}

export function clearSessionCookie(cookies: Cookies) {
	cookies.delete(SETUP_SESSION_COOKIE, { path: '/' });
}
