// 802.1X settings in Tetra's shape, shared by `network.configure`'s
// `wired_8021x` and `wpa-enterprise` Wi-Fi in `network.connect_wifi`. Tetra
// refuses unknown fields, so only these names are ever sent. Certificates and
// keys are PEM contents; Tetra stores them root-only and refers to them by path.

export const EAP_METHODS = ['tls', 'peap', 'ttls', 'pwd', 'md5'] as const;
export const PHASE2_METHODS = ['mschapv2', 'mschap', 'pap', 'chap', 'gtc', 'md5'] as const;

export type EapMethod = (typeof EAP_METHODS)[number];
export type Phase2Method = (typeof PHASE2_METHODS)[number];

export type Eap8021x = {
	eap: EapMethod;
	identity: string;
	anonymous_identity?: string;
	password?: string;
	ca_cert?: string;
	client_cert?: string;
	private_key?: string;
	private_key_password?: string;
	domain_suffix_match?: string;
	phase2?: Phase2Method;
};

/** What the 802.1X form fields hold. File names are only shown, never sent. */
export type Eap8021xForm = {
	eap: EapMethod;
	phase2: Phase2Method;
	identity: string;
	anonymousIdentity: string;
	password: string;
	domain: string;
	caCert: string;
	caCertName: string;
	clientCert: string;
	clientCertName: string;
	privateKey: string;
	privateKeyName: string;
	privateKeyPassword: string;
};

export function emptyEap8021xForm(): Eap8021xForm {
	return {
		eap: 'peap',
		phase2: 'mschapv2',
		identity: '',
		anonymousIdentity: '',
		password: '',
		domain: '',
		caCert: '',
		caCertName: '',
		clientCert: '',
		clientCertName: '',
		privateKey: '',
		privateKeyName: '',
		privateKeyPassword: ''
	};
}

/** Tunneled methods carry an inner (phase 2) method and an outer identity. */
export function isTunneled(eap: EapMethod): boolean {
	return eap === 'peap' || eap === 'ttls';
}

function needsPassword(eap: EapMethod): boolean {
	return eap !== 'tls';
}

/** Build Tetra's settings from the form, leaving out fields the method does not use. */
export function eap8021xSettings(form: Eap8021xForm): Eap8021x {
	const optional = (value: string) => value.trim() || undefined;
	// PEM contents go to Tetra exactly as read from the file.
	const pem = (value: string) => (value.trim() ? value : undefined);
	const tunneled = isTunneled(form.eap);
	const tls = form.eap === 'tls';
	return {
		eap: form.eap,
		identity: form.identity.trim(),
		anonymous_identity: tunneled ? optional(form.anonymousIdentity) : undefined,
		password: needsPassword(form.eap) ? form.password : undefined,
		ca_cert: pem(form.caCert),
		client_cert: tls ? pem(form.clientCert) : undefined,
		private_key: tls ? pem(form.privateKey) : undefined,
		private_key_password: tls ? form.privateKeyPassword || undefined : undefined,
		domain_suffix_match: optional(form.domain)?.toLowerCase(),
		phase2: tunneled ? form.phase2 : undefined
	};
}

const DOMAIN_SUFFIX_PATTERN =
	/^(?=.{1,253}$)[a-z0-9]([a-z0-9-]{0,61}[a-z0-9])?(\.[a-z0-9]([a-z0-9-]{0,61}[a-z0-9])?)*$/;

function isPem(value: string): boolean {
	return value.trimStart().startsWith('-----BEGIN ') && value.includes('-----END ');
}

const FIELDS: (keyof Eap8021x)[] = [
	'eap',
	'identity',
	'anonymous_identity',
	'password',
	'ca_cert',
	'client_cert',
	'private_key',
	'private_key_password',
	'domain_suffix_match',
	'phase2'
];

const PEM_FIELDS = {
	ca_cert: 'CA certificate',
	client_cert: 'Client certificate',
	private_key: 'Private key'
} as const;

/**
 * Check 802.1X settings and return them with only Tetra's fields, or an error
 * message. `extra` names fields the caller handles itself (such as `type`).
 */
export function validateEap8021x(value: unknown, extra: string[] = []): Eap8021x | string {
	if (!value || typeof value !== 'object' || Array.isArray(value))
		return '802.1X settings are required';
	const input = value as Record<string, unknown>;
	for (const key of Object.keys(input)) {
		if (!FIELDS.includes(key as keyof Eap8021x) && !extra.includes(key))
			return `Unknown 802.1X setting: ${key}`;
	}
	for (const key of FIELDS) {
		if (input[key] !== undefined && typeof input[key] !== 'string')
			return `Invalid 802.1X setting: ${key}`;
	}
	const eap = input.eap as EapMethod;
	if (!EAP_METHODS.includes(eap)) return 'Unsupported EAP method';
	const identity = ((input.identity as string | undefined) ?? '').trim();
	if (!identity) return 'Identity is required';
	if (needsPassword(eap) && !input.password) return 'Password is required';
	if (eap === 'tls' && (!input.client_cert || !input.private_key))
		return 'EAP-TLS needs a client certificate and private key';
	for (const [key, label] of Object.entries(PEM_FIELDS)) {
		const pem = input[key] as string | undefined;
		if (pem !== undefined && !isPem(pem)) return `${label} must be a PEM file`;
	}
	const phase2 = input.phase2 as Phase2Method | undefined;
	if (phase2 !== undefined && !PHASE2_METHODS.includes(phase2))
		return 'Unsupported inner authentication method';
	const domain = (input.domain_suffix_match as string | undefined)?.trim().toLowerCase();
	if (domain !== undefined && !DOMAIN_SUFFIX_PATTERN.test(domain))
		return 'Enter a domain such as example.com';

	const settings: Eap8021x = { eap, identity };
	for (const key of FIELDS) {
		if (key === 'eap' || key === 'identity' || input[key] === undefined) continue;
		(settings as Record<string, unknown>)[key] = input[key];
	}
	if (domain !== undefined) settings.domain_suffix_match = domain;
	return settings;
}
