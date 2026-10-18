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

let fixtureAuthenticated = false;

/** Fixture API for development without a backend. Accepts `FIXTURE_SETUP_CODE`. */
export const fixtureApi: OobeApi = {
	async getSession() {
		return { authenticated: fixtureAuthenticated, setupRequired: true };
	},
	async redeemSetupCode(code) {
		const normalize = (value: string) => value.toUpperCase().replace(/[^0-9A-Z]/g, '');
		fixtureAuthenticated = normalize(code) === normalize(FIXTURE_SETUP_CODE);
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
		if (operation === 'network.scan_wifi') {
			return {
				id: `fixture-${operation}`,
				step,
				status: 'succeeded',
				retryable: false,
				message: JSON.stringify([
					{ ssid: 'Ultramarine-Setup', security: 'wpa-personal', signal: 86 },
					{
						ssid: 'Corp Wi-Fi',
						security: 'wpa-enterprise',
						signal: 72,
						eapMethods: ['peap', 'tls'],
						phase2Methods: ['mschapv2', 'pap']
					},
					{ ssid: 'Guest Network', security: 'open', signal: 58 }
				])
			};
		}
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
		if (operation === 'network.interfaces') {
			return {
				id: `fixture-${operation}`,
				step,
				status: 'succeeded',
				retryable: false,
				message: JSON.stringify(fixtureState.networkInterfaces ?? [])
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
