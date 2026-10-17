export type StepId =
	| 'welcome'
	| 'devicename'
	| 'whoareyou'
	| 'password'
	| 'internet'
	| 'tweaks'
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

export type DashboardState = {
	installed: boolean;
	installing: boolean;
};

/** Compatibility state for legacy operation endpoints; not part of the setup flow. */
export type LegacyTetraState = { installed: boolean; running: boolean; paired: boolean };
export type LegacyFyraState = {
	status: string;
	verificationUri?: string;
	userCode?: string;
	message?: string;
};
export type LegacyCloudflareState = { installed: boolean; installing: boolean };

export type NetworkInterface = {
	name: string;
	state?: string;
	addresses?: string[];
	wireless?: boolean;
};

export type WifiSecurity = 'open' | 'wpa-personal' | 'wpa-enterprise' | 'unknown';

export type WifiNetwork = {
	ssid: string;
	security: WifiSecurity;
	signal?: number;
	eapMethods?: string[];
	phase2Methods?: string[];
};

export type WifiSecurityProfile = {
	type: WifiSecurity;
	password?: string;
	eap?: string;
	phase2?: string;
	domain?: string;
	username?: string;
	anonymousIdentity?: string;
	caCertificate?: string;
	caCertificateName?: string;
	clientCertificate?: string;
	privateKey?: string;
	privateKeyPassword?: string;
};

export type OobeState = {
	version: 1;
	completed: boolean;
	activeStep: StepId;
	steps: OobeStep[];
	hostname: string;
	administrator: string;
	keyboardLayout?: string;
	keyboardVariant?: string;
	tweaks?: {
		ssh: boolean;
		firewall: boolean;
		automaticUpdates: boolean;
	};
	sshPublicKey?: string;
	networkInterfaces?: NetworkInterface[];
	dashboardDomain: string;
	dashboardPort: number;
	dashboard: DashboardState;
	tetra: LegacyTetraState;
	fyra: LegacyFyraState;
	cloudflare: LegacyCloudflareState;
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
		id: 'tweaks',
		label: 'step-tweaks-label',
		status: 'pending',
		description: 'step-tweaks-description'
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
	tweaks: { ssh: true, firewall: true, automaticUpdates: true },
	sshPublicKey: '',
	networkInterfaces: [
		{ name: 'enp1s0', state: 'connected', addresses: ['192.168.1.42'], wireless: false },
		{ name: 'wlan0', state: 'disconnected', addresses: [], wireless: true }
	],
	dashboardDomain: '',
	dashboardPort: 3972,
	dashboard: { installed: false, installing: false },
	tetra: { installed: true, running: true, paired: false },
	fyra: { status: 'not-started' },
	cloudflare: { installed: false, installing: false }
};

export function stepIndex(id: StepId): number {
	return fixtureSteps.findIndex((step) => step.id === id);
}
