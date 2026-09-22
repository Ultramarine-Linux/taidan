import { fixtureState, type OobeState, type OperationResult, type StepId } from './oobe-state';

export interface OobeApi {
	getState(): Promise<OobeState>;
	completeStep(step: StepId): Promise<OperationResult>;
	startOperation(
		step: StepId,
		operation: string,
		payload?: Record<string, unknown>
	): Promise<OperationResult>;
}

export const liveApi: OobeApi = {
	async getState() {
		const res = await fetch('/api/oobe/state');
		if (!res.ok) throw new Error(`Failed to load state: ${res.status}`);
		return (await res.json()) as OobeState;
	},

	async completeStep(step) {
		const res = await fetch('/api/oobe/state', {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ steps: [{ id: step, status: 'complete' }] })
		});
		if (!res.ok) throw new Error(`Failed to complete step: ${res.status}`);
		return { id: `complete-${step}`, step, status: 'succeeded', retryable: false };
	},

	async startOperation(step, operation, payload = {}) {
		const res = await fetch('/api/oobe/operations', {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ step, operation, payload })
		});
		if (!res.ok) throw new Error(`Operation failed: ${res.status}`);
		return (await res.json()) as OperationResult;
	}
};

/** Fixture API for development without a backend. */
export const fixtureApi: OobeApi = {
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
