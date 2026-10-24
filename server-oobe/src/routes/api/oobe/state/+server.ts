import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { loadState, saveState } from '$lib/server/state';

export const GET: RequestHandler = async () => json(await loadState());

export const POST: RequestHandler = async ({ request }) => {
	const body = (await request.json()) as {
		steps?: Array<{ id: string; status: string }>;
	};
	const state = await loadState();
	if (Array.isArray(body.steps)) {
		state.steps = state.steps.map((current) => {
			const update = body.steps?.find((candidate) => candidate.id === current.id);
			return update ? { ...current, status: update.status as typeof current.status } : current;
		});
	}
	await saveState(state);
	return json(state);
};
