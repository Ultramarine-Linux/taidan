import type { Eap8021x } from './eap-8021x';
import { fixtureHost } from './fixture-hosts';
import type { InterfaceAddress, InterfaceType } from './network-config';

export type StepId =
	| 'welcome'
	| 'devicename'
	| 'whoareyou'
	| 'password'
	| 'internet'
	| 'updates'
	| 'remote-access'
	| 'ssh-key'
	| 'dashboard'
	| 'complete';

export type StepStatus =
	'pending' | 'active' | 'complete' | 'requires-authentication' | 'blocked' | 'failed';

export type OobeStep = {
	id: StepId;
	label: string;
	status: StepStatus;
	description: string;
};

/** How the server applies updates; Tetra decides what that means on bootc or mutable installs. */
export type UpdatePolicy = 'automatic' | 'notify';

export type AccessMode = 'domain' | 'ip';
export type AcmeChallenge = 'http-01' | 'dns-01';

/** How browsers reach the dashboard. DNS provider credentials are never stored. */
export type AccessState = {
	mode: AccessMode;
	hostname: string;
	challenge: AcmeChallenge;
	dnsProvider?: string;
	/** Set once Tetra has accepted the configuration. */
	applied: boolean;
	passkeysAvailable?: boolean;
};

/** Which remote access services this setup enabled. Keys and tokens are never stored. */
export type RemoteAccessState = {
	tailscale: boolean;
	tailscaleServe: boolean;
	tailscaleFunnel: boolean;
	cloudflared: boolean;
};

/**
 * Remote access choices in the browser, secrets included. Lives only in page
 * memory: it is sent to Tetra once and never persisted.
 */
export type RemoteAccessForm = {
	tailscale: boolean;
	tailscaleAuthKey: string;
	tailscaleServe: boolean;
	tailscaleFunnel: boolean;
	funnelConfirmed: boolean;
	cloudflared: boolean;
	cloudflaredToken: string;
};

/** Access choices in the browser. DNS credentials live only in page memory. */
export type AccessForm = {
	mode: AccessMode;
	hostname: string;
	challenge: AcmeChallenge;
	dnsProvider: string;
	dnsCredentials: Record<string, string>;
};

/** Result of setup.complete, shown on the final screen. */
export type SetupCompleteResult = {
	dashboardUrl?: string;
	handoffInSeconds: number;
};

/** An interface from Tetra's `network.interfaces`, without its MAC address. */
export type NetworkInterface = {
	name: string;
	/** Missing from older Tetra; see `interfaceType`. */
	type?: InterfaceType;
	operstate?: string;
	addresses?: InterfaceAddress[];
};

export type WifiSecurity = 'open' | 'wpa-personal' | 'wpa-enterprise' | 'unknown';

/** A network from Tetra's `network.scan_wifi`. */
export type WifiNetwork = {
	ssid: string;
	security: WifiSecurity;
	signal?: number;
	frequency?: number;
	in_use?: boolean;
};

/** Result of the `network.check` operation. */
export type NetworkCheck = { ipv4: boolean; ipv6: boolean; dns: boolean };

/**
 * A network change being applied (`network.configure` or
 * `network.connect_wifi`), shown while the browser may lose its connection.
 */
export type NetworkApplyState = {
	status: 'applying' | 'waiting' | 'kept' | 'rolled-back' | 'failed';
	/** Where the OOBE should be reachable afterwards. */
	urls: { label: string; href: string }[];
	/** Some addresses come from DHCP or SLAAC and may differ from `urls`. */
	automatic: boolean;
	/** Seconds Tetra waits for a confirmation before it rolls the change back. */
	rollbackInSeconds: number;
	message?: string;
};

/** `security` for Tetra's `network.connect_wifi`, tagged by `type`. */
export type WifiSecurityProfile =
	| { type: 'open' }
	| { type: 'wpa-personal'; password: string }
	| ({ type: 'wpa-enterprise' } & Eap8021x);

export type OobeState = {
	version: 1;
	completed: boolean;
	activeStep: StepId;
	steps: OobeStep[];
	hostname: string;
	administrator: string;
	keyboardLayout?: string;
	keyboardVariant?: string;
	updatePolicy: UpdatePolicy;
	sshPublicKey?: string;
	networkInterfaces?: NetworkInterface[];
	access: AccessState;
	remoteAccess: RemoteAccessState;
};

export const defaultAccess: AccessState = {
	mode: 'domain',
	hostname: '',
	challenge: 'http-01',
	applied: false
};

export const defaultRemoteAccess: RemoteAccessState = {
	tailscale: false,
	tailscaleServe: true,
	tailscaleFunnel: false,
	cloudflared: false
};

/** Response of /api/oobe/session. The setup session token itself stays in an HttpOnly cookie. */
export type SessionResponse = {
	authenticated: boolean;
	setupRequired: boolean;
	adminExists?: boolean;
	error?: 'invalid_code' | 'rate_limited' | 'setup_complete' | 'unavailable';
	retryAfter?: number;
	message?: string;
};

export type OperationStatus = 'pending' | 'running' | 'succeeded' | 'failed';
export type OperationResult = {
	id: string;
	step: StepId;
	status: OperationStatus;
	retryable: boolean;
	message?: string;
};

export const fixtureSteps: OobeStep[] = [
	{
		id: 'welcome',
		label: 'step-welcome-label',
		status: 'active',
		description: 'step-welcome-description'
	},
	{
		id: 'devicename',
		label: 'step-device-name-label',
		status: 'pending',
		description: 'step-device-name-description'
	},
	{
		id: 'whoareyou',
		label: 'step-administrator-label',
		status: 'pending',
		description: 'step-administrator-description'
	},
	{
		id: 'password',
		label: 'step-password-label',
		status: 'pending',
		description: 'step-password-description'
	},
	{
		id: 'ssh-key',
		label: 'step-ssh-key-label',
		status: 'pending',
		description: 'step-ssh-key-description'
	},
	{
		id: 'internet',
		label: 'step-internet-label',
		status: 'pending',
		description: 'step-internet-description'
	},
	{
		id: 'updates',
		label: 'step-updates-label',
		status: 'pending',
		description: 'step-updates-description'
	},
	{
		id: 'remote-access',
		label: 'step-remote-access-label',
		status: 'pending',
		description: 'step-remote-access-description'
	},
	{
		id: 'dashboard',
		label: 'step-dashboard-label',
		status: 'pending',
		description: 'step-dashboard-description'
	},
	{
		id: 'complete',
		label: 'step-complete-label',
		status: 'pending',
		description: 'step-complete-description'
	}
];

export const fixtureState: OobeState = {
	version: 1,
	completed: false,
	activeStep: 'welcome',
	steps: fixtureSteps,
	hostname: '',
	administrator: '',
	keyboardLayout: undefined,
	keyboardVariant: undefined,
	updatePolicy: 'automatic',
	sshPublicKey: '',
	networkInterfaces: fixtureHost().interfaces.filter((entry) => entry.name !== 'lo'),
	access: { ...defaultAccess },
	remoteAccess: { ...defaultRemoteAccess }
};

export function stepIndex(id: StepId): number {
	return fixtureSteps.findIndex((step) => step.id === id);
}
