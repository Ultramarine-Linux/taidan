// DNS providers for ACME DNS-01, limited to the caddy-dns modules built into
// terra-caddy. Credential field names follow each module's Caddy JSON config;
// the OOBE passes them to Tetra's access.set unchanged and never stores them.

export type DnsCredentialField = {
	/** Caddy JSON field name, sent to Tetra as a `dns_credentials` key. */
	name: string;
	label: string;
	secret: boolean;
	optional?: boolean;
};

export type DnsProvider = {
	/** Caddy `dns.providers.<id>` module name. */
	id: string;
	label: string;
	fields: DnsCredentialField[];
};

export const DNS_PROVIDERS: DnsProvider[] = [
	{
		id: 'cloudflare',
		label: 'Cloudflare',
		fields: [{ name: 'api_token', label: 'API token', secret: true }]
	},
	{
		id: 'route53',
		label: 'Amazon Route 53',
		fields: [
			{ name: 'access_key_id', label: 'Access key ID', secret: false },
			{ name: 'secret_access_key', label: 'Secret access key', secret: true },
			{ name: 'region', label: 'Region', secret: false, optional: true }
		]
	},
	{
		id: 'desec',
		label: 'deSEC',
		fields: [{ name: 'token', label: 'Token', secret: true }]
	},
	{
		id: 'gandi',
		label: 'Gandi',
		fields: [{ name: 'bearer_token', label: 'Personal access token', secret: true }]
	},
	{
		id: 'hetzner',
		label: 'Hetzner',
		fields: [{ name: 'api_token', label: 'API token', secret: true }]
	},
	{
		id: 'linode',
		label: 'Linode (Akamai)',
		fields: [{ name: 'api_token', label: 'API token', secret: true }]
	},
	{
		id: 'ovh',
		label: 'OVHcloud',
		fields: [
			{ name: 'endpoint', label: 'Endpoint (for example ovh-eu)', secret: false },
			{ name: 'application_key', label: 'Application key', secret: false },
			{ name: 'application_secret', label: 'Application secret', secret: true },
			{ name: 'consumer_key', label: 'Consumer key', secret: true }
		]
	},
	{
		id: 'porkbun',
		label: 'Porkbun',
		fields: [
			{ name: 'api_key', label: 'API key', secret: true },
			{ name: 'api_secret_key', label: 'Secret API key', secret: true }
		]
	},
	{
		id: 'powerdns',
		label: 'PowerDNS',
		fields: [
			{ name: 'server_url', label: 'Server URL', secret: false },
			{ name: 'api_token', label: 'API key', secret: true }
		]
	},
	{
		id: 'rfc2136',
		label: 'RFC 2136 (dynamic DNS update)',
		fields: [
			{ name: 'server', label: 'Server (host:port)', secret: false },
			{ name: 'key_name', label: 'TSIG key name', secret: false },
			{ name: 'key_alg', label: 'TSIG algorithm (for example hmac-sha256)', secret: false },
			{ name: 'key', label: 'TSIG secret', secret: true }
		]
	},
	{
		id: 'acmedns',
		label: 'ACME-DNS',
		fields: [
			{ name: 'server_url', label: 'Server URL', secret: false },
			{ name: 'subdomain', label: 'Subdomain', secret: false },
			{ name: 'username', label: 'Username', secret: false },
			{ name: 'password', label: 'Password', secret: true }
		]
	}
];

export function dnsProvider(id: string): DnsProvider | undefined {
	return DNS_PROVIDERS.find((provider) => provider.id === id);
}

/**
 * Check credentials against a provider's fields. Returns an error message, or
 * undefined when they are complete. Unknown fields are rejected.
 */
export function validateDnsCredentials(
	providerId: string,
	credentials: unknown
): string | undefined {
	const provider = dnsProvider(providerId);
	if (!provider) return 'Unsupported DNS provider';
	if (!credentials || typeof credentials !== 'object' || Array.isArray(credentials))
		return 'DNS provider credentials are required';
	const values = credentials as Record<string, unknown>;
	for (const key of Object.keys(values)) {
		if (!provider.fields.some((field) => field.name === key))
			return `Unknown credential for ${provider.label}`;
		if (typeof values[key] !== 'string') return `Invalid credential for ${provider.label}`;
	}
	for (const field of provider.fields) {
		const value = values[field.name];
		if (!field.optional && (typeof value !== 'string' || !value.trim()))
			return `${field.label} is required`;
	}
	return undefined;
}
