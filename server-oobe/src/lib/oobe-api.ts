import {
	fixtureState,
	type OobeState,
	type OperationResult,
	type SessionResponse,
	type StepId
} from './oobe-state';
import { FIXTURE_SETUP_CODE } from './fixture';

/** Thrown when the server has no live setup session for this browser. */
export class SetupSessionRequiredError extends Error {
	constructor() {
		super('Enter the setup code shown on the server console to continue.');
		this.name = 'SetupSessionRequiredError';
	}
}

function checkSession(res: Response) {
	if (res.status === 401) throw new SetupSessionRequiredError();
}

export interface OobeApi {
	getSession(): Promise<SessionResponse>;
	/** Redeem the setup code from the server console. Tetra checks it. */
	redeemSetupCode(code: string): Promise<SessionResponse>;
	getState(): Promise<OobeState>;
	completeStep(step: StepId): Promise<OperationResult>;
	startOperation(
		step: StepId,
		operation: string,
		payload?: Record<string, unknown>
	): Promise<OperationResult>;
}

export const liveApi: OobeApi = {
	async getSession() {
		const res = await fetch('/api/oobe/session');
		return (await res.json()) as SessionResponse;
	},

	async redeemSetupCode(code) {
		const res = await fetch('/api/oobe/session', {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ code })
		});
		return (await res.json()) as SessionResponse;
	},

	async getState() {
		const res = await fetch('/api/oobe/state');
		checkSession(res);
		if (!res.ok) throw new Error(`Failed to load state: ${res.status}`);
		return (await res.json()) as OobeState;
	},

	async completeStep(step) {
		const res = await fetch('/api/oobe/state', {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ steps: [{ id: step, status: 'complete' }] })
		});
		checkSession(res);
		if (!res.ok) throw new Error(`Failed to complete step: ${res.status}`);
		return { id: `complete-${step}`, step, status: 'succeeded', retryable: false };
	},

	async startOperation(step, operation, payload = {}) {
		const res = await fetch('/api/oobe/operations', {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ step, operation, payload })
		});
		checkSession(res);
		if (!res.ok) throw new Error(`Operation failed: ${res.status}`);
		return (await res.json()) as OperationResult;
	}
};

export type NetworkChangeStatus = 'kept' | 'rolled-back' | 'failed' | 'unknown';

/**
 * Ask the OOBE to keep the network change `changeId` (see
 * src/lib/server/network-change.ts). Resolves to undefined when the OOBE
 * cannot be reached, which is expected while the host changes addresses.
 */
export async function confirmNetworkChange(
	changeId: string,
	timeoutMs = 5000
): Promise<{ status: NetworkChangeStatus; message?: string } | undefined> {
	try {
		const res = await fetch('/api/oobe/network/confirm', {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ changeId }),
			signal: AbortSignal.timeout(timeoutMs)
		});
		if (res.status >= 500) return undefined;
		return (await res.json()) as { status: NetworkChangeStatus; message?: string };
	} catch {
		return undefined;
	}
}

let fixtureAuthenticated = false;

/**
 * Fixture API for development: Welcome accepts `FIXTURE_SETUP_CODE` and most
 * steps only pretend. Network operations go to the dev server, which runs them
 * against the mock Tetra, so the internet step sees the fixture host picked by
 * the `oobe_fixture_host` cookie (src/lib/fixture-hosts.ts).
 */
export const fixtureApi: OobeApi = {
	async getSession() {
		return { authenticated: fixtureAuthenticated, setupRequired: true };
	},
	async redeemSetupCode(code) {
		const normalize = (value: string) => value.toUpperCase().replace(/[^0-9A-Z]/g, '');
		fixtureAuthenticated = normalize(code) === normalize(FIXTURE_SETUP_CODE);
		// A setup session with the mock Tetra, for the network operations.
		if (fixtureAuthenticated) await liveApi.redeemSetupCode(code);
		return fixtureAuthenticated
			? { authenticated: true, setupRequired: true, adminExists: false }
			: { authenticated: false, setupRequired: true, error: 'invalid_code' };
	},
	async getState() {
		return structuredClone(fixtureState);
	},
	async completeStep(step) {
		return { id: `fixture-${step}`, step, status: 'succeeded', retryable: false };
	},
	async startOperation(step, operation, payload = {}) {
		if (operation.startsWith('network.')) return liveApi.startOperation(step, operation, payload);
		const fixtureResults: Record<string, unknown> = {
			'access.set': { passkeysAvailable: payload.mode === 'domain' },
			'setup.complete': {
				dashboardUrl: payload.hostname
					? `https://${payload.hostname}`
					: 'https://server.example.com',
				handoffInSeconds: 5
			},
			'remote_access.status': {
				tailscale: { enabled: false, serve: false, funnel: false },
				cloudflared: { enabled: false }
			}
		};
		if (operation in fixtureResults) {
			return {
				id: `fixture-${operation}`,
				step,
				status: 'succeeded',
				retryable: false,
				message: JSON.stringify(fixtureResults[operation])
			};
		}
		return {
			id: `fixture-${operation}`,
			step,
			status: 'succeeded',
			retryable: false,
			message: `Fixture mode: ${operation}${Object.keys(payload).length ? ' accepted' : ''}`
		};
	}
};

export const api: OobeApi = import.meta.env.VITE_FIXTURE_API === 'true' ? fixtureApi : liveApi;
