// Safe apply for `network.configure` and `network.connect_wifi`.
//
// A change can move the host to addresses the browser is not using. When none
// of the interface's old addresses come back, Tetra answers with a
// confirmation token and rolls the change back unless `network.confirm`
// follows within `rollback_in_seconds` (60). The browser may never see that
// answer, so the OOBE keeps it here under an id the browser chose before it
// sent the change. The browser (or a new tab at the new address, with the id
// in its URL) then asks /api/oobe/network/confirm to keep it as soon as it can
// reach the OOBE again; reaching it is the proof Tetra wants.
//
// This module must not import SvelteKit aliases ($lib, $app, $env) so that
// tests can load it directly.

import type { TetraResponse } from './tetra';

/**
 * Browser-chosen change id, base64url. 22 characters carry 132 bits; the
 * browser sends 24 random ones (144 bits). The server cannot check that they
 * are random, but a weak id only exposes the change of the session that chose
 * it, and only to being kept.
 */
export const CHANGE_ID_PATTERN = /^[A-Za-z0-9_-]{22,64}$/;

/**
 * How long a settled change's outcome is kept, so a late confirm (say, from
 * the original tab once it can reach the OOBE again) learns how it ended.
 */
export const KEEP_OUTCOME_MS = 5 * 60 * 1000;

export type NetworkChangeStatus = 'kept' | 'rolled-back' | 'failed' | 'unknown';

type Change = {
	/** Settles once Tetra has answered the change. */
	applied: Promise<void>;
	/** The setup session that made the change; `network.confirm` runs with it. */
	sessionToken?: string;
	confirmationToken?: string;
	/** When Tetra rolls the change back (ms since the epoch). */
	rollbackAt?: number;
	/** A `network.confirm` call in flight, shared by concurrent confirms. */
	confirming?: Promise<void>;
	status?: Exclude<NetworkChangeStatus, 'unknown'>;
	message?: string;
	/** How long the outcome is kept once the change settles. */
	keepOutcomeMs: number;
	/** Drops the outcome `keepOutcomeMs` after the change settled. */
	forget?: ReturnType<typeof setTimeout>;
};

const changes = new Map<string, Change>();

/**
 * Record how `change` ended, drop its tokens, and forget it `keepOutcomeMs`
 * later. A confirm Tetra accepted after the rollback timer fired wins.
 */
function settle(
	changeId: string,
	change: Change,
	status: Exclude<NetworkChangeStatus, 'unknown'>,
	message?: string
) {
	change.status = status;
	change.message = message;
	change.sessionToken = undefined;
	change.confirmationToken = undefined;
	change.forget ??= setTimeout(() => {
		if (changes.get(changeId) === change) changes.delete(changeId);
	}, change.keepOutcomeMs);
	change.forget.unref?.();
}

/**
 * Run a change and remember Tetra's answer under `changeId`. Resolves to
 * Tetra's response; a confirmation it asks for waits for `confirmChange`.
 * An id names exactly one change: a known id is refused without running.
 *
 * The id can only keep this change, at most once, and only until Tetra's
 * rollback; after that it only reports how the change ended, until the
 * outcome is dropped `keepOutcomeMs` after it settled.
 */
export function applyChange(
	changeId: string,
	sessionToken: string,
	run: () => Promise<TetraResponse>,
	keepOutcomeMs: number = KEEP_OUTCOME_MS
): Promise<TetraResponse> {
	if (changes.has(changeId)) {
		return Promise.resolve({ ok: false, error: 'This change id was already used' });
	}
	const change: Change = { applied: Promise.resolve(), keepOutcomeMs };
	const done = (status: Exclude<NetworkChangeStatus, 'unknown'>, message?: string) =>
		settle(changeId, change, status, message);
	const response = run().then(
		(resp) => {
			const payload = resp.payload ?? {};
			if (!resp.ok) {
				done('failed', resp.error);
			} else if (
				payload.confirmation_required === true &&
				typeof payload.confirmation_token === 'string'
			) {
				const seconds =
					typeof payload.rollback_in_seconds === 'number' ? payload.rollback_in_seconds : 60;
				change.sessionToken = sessionToken;
				change.confirmationToken = payload.confirmation_token;
				change.rollbackAt = Date.now() + seconds * 1000;
				// Tetra has rolled back by then; the id expires with it.
				setTimeout(() => {
					if (!change.status) done('rolled-back');
				}, seconds * 1000).unref?.();
			} else {
				done('kept');
			}
			return resp;
		},
		(error: unknown) => {
			done('failed', error instanceof Error ? error.message : String(error));
			throw error;
		}
	);
	change.applied = response.then(
		() => undefined,
		() => undefined
	);
	changes.set(changeId, change);
	return response;
}

/**
 * Keep the change `changeId` if Tetra is waiting for a confirmation. Waits for
 * a change that is still being applied.
 */
export async function confirmChange(
	changeId: string,
	confirm: (sessionToken: string, confirmationToken: string) => Promise<TetraResponse>
): Promise<{ status: NetworkChangeStatus; message?: string }> {
	const change = changes.get(changeId);
	if (!change) return { status: 'unknown' };
	await change.applied;
	const { sessionToken, confirmationToken } = change;
	if (!change.status && sessionToken && confirmationToken) {
		change.confirming ??= (async () => {
			if ((change.rollbackAt ?? 0) <= Date.now()) {
				settle(changeId, change, 'rolled-back');
				return;
			}
			const resp = await confirm(sessionToken, confirmationToken);
			if (resp.ok) settle(changeId, change, 'kept');
			else change.message = resp.error;
		})().finally(() => (change.confirming = undefined));
		await change.confirming;
	}
	// Until it settles (a failed confirm can be retried), the browser keeps asking.
	return { status: change.status ?? 'unknown', message: change.message };
}
