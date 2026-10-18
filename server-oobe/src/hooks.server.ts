import { json, type Handle } from '@sveltejs/kit';
import { checkSetupSession, clearSessionCookie } from '$lib/server/setup-session';

/**
 * API routes that work without a setup session: redeeming the setup code, and
 * the address list the welcome screen shows before the code is entered.
 */
const PUBLIC_API = new Set(['/api/oobe/session', '/api/oobe/network']);

export const handle: Handle = async ({ event, resolve }) => {
	const path = event.url.pathname;
	if (path.startsWith('/api/') && !PUBLIC_API.has(path)) {
		const session = await checkSetupSession(event.cookies);
		if (session.status === 'unavailable') {
			return json({ message: `Tetra is unavailable: ${session.message}` }, { status: 503 });
		}
		if (session.status === 'missing') {
			clearSessionCookie(event.cookies);
			return json(
				{ message: 'Enter the setup code shown on the server console to continue.' },
				{ status: 401 }
			);
		}
		event.locals.setupToken = session.token;
	}
	return resolve(event);
};
