// Types shared by Tetra's local session API client and its mock. See
// tetra.ts.
//
// This module must not import SvelteKit aliases ($lib, $app, $env) so that
// tests can load it directly.

import type { AcmeChallenge, UpdatePolicy } from '../oobe-state';

/** Protocol version this client was written against. */
export const TETRA_PROTOCOL_VERSION = '2026-10-session-v1';
export const DEFAULT_TETRA_SOCKET = '/run/tetra/tetra.sock';

export type TetraErrorCode =
	| 'bad_request'
	| 'unknown_operation'
	| 'unauthenticated'
	| 'forbidden'
	| 'invalid_credentials'
	| 'password_expired'
	| 'rate_limited'
	| 'passkeys_unavailable'
	| 'wrong_client'
	| 'setup_complete'
	| 'setup_incomplete'
	| 'not_found'
	| 'conflict'
	| 'action_failed'
	| 'queue_full'
	| 'internal'
	/** Tetra could not be reached or answered with something unexpected. */
	| 'unavailable';

export class TetraError extends Error {
	constructor(
		readonly code: TetraErrorCode,
		message: string,
		readonly retryAfter?: number
	) {
		super(message);
		this.name = 'TetraError';
	}
}

export interface SetupStatus {
	setupRequired: boolean;
	passkeysAvailable: boolean;
}

export interface SetupSession {
	token: string;
	/** Unix seconds. */
	expiresAt: number;
	/** Whether an eligible administrator already exists (for example from Readymade). */
	adminExists: boolean;
}

export interface SessionInfo {
	kind: 'setup' | 'user';
	client: 'oobe' | 'dashboard';
	/** Unix seconds. */
	expiresAt: number;
}

// The request and result shapes below for access.set (with DNS-01),
// setup.complete (with the dashboard handoff) and the settings, users, power,
// updates and remote_access actions are the agreed interface: Tetra implements
// them exactly as written here, and the mock follows them until it does.

/** `access.set` params. DNS-01 credentials pass straight through to Tetra. */
export type AccessRequest =
	| { mode: 'ip'; hostname: string }
	| { mode: 'domain'; hostname: string; challenge: Extract<AcmeChallenge, 'http-01'> }
	| {
			mode: 'domain';
			hostname: string;
			challenge: Extract<AcmeChallenge, 'dns-01'>;
			dns_provider: string;
			dns_credentials: Record<string, string>;
	  };

export interface AccessResult {
	passkeysAvailable: boolean;
	/** `https://hostname[:port]`. */
	dashboardUrl?: string;
}

export interface SetupCompleteResult {
	/** Where the dashboard will be reachable, from the access configuration. */
	dashboardUrl?: string;
	/** Seconds until Tetra stops the OOBE. */
	handoffInSeconds: number;
}

/** `updates.set_policy` payload. */
export interface UpdatePolicyRequest {
	policy: UpdatePolicy;
}

/** `remote_access.tailscale_up` payload. */
export interface TailscaleUpRequest {
	auth_key: string;
	/** Share the dashboard on the tailnet with `tailscale serve`. */
	serve: boolean;
	/** Publish the dashboard on the internet with `tailscale funnel`. */
	funnel: boolean;
}

/** `remote_access.cloudflared_set_token` payload. */
export interface CloudflaredTokenRequest {
	token: string;
}

/** `remote_access.status` result. */
export interface RemoteAccessStatus {
	tailscale: { enabled: boolean; serve: boolean; funnel: boolean; hostname?: string };
	cloudflared: { enabled: boolean };
}

/** The parts of Tetra's local API the OOBE uses. */
export interface TetraClient {
	/** `setup.status`, no token. */
	setupStatus(): Promise<SetupStatus>;
	/**
	 * `setup.redeem`, no token. Fails with `invalid_credentials`, `rate_limited`
	 * or `setup_complete`. `remoteAddr` is the browser's address (as Caddy saw
	 * it), sent as `remote_addr` like `auth.password`; Tetra logs it and
	 * throttles per address on top of its global setup-code counter.
	 */
	redeemSetupCode(code: string, remoteAddr?: string): Promise<SetupSession>;
	/** `session.info`. Fails with `unauthenticated` for a missing, expired or revoked token. */
	sessionInfo(token: string): Promise<SessionInfo>;
	/** `access.set`. Tetra writes the dashboard's Caddy site and reloads Caddy. */
	setAccess(token: string, request: AccessRequest): Promise<AccessResult>;
	/**
	 * `setup.complete`. Ends every setup session, disables the setup code,
	 * starts the dashboard and stops the OOBE a few seconds later.
	 */
	completeSetup(token: string): Promise<SetupCompleteResult>;
	/** `module.dispatch`. Resolves to the action's result payload. */
	dispatch(
		token: string,
		module: string,
		action: string,
		payload?: Record<string, unknown>
	): Promise<Record<string, unknown>>;
}
