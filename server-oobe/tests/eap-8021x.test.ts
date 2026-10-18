// 802.1X settings in Tetra's field names, for wired_8021x and wpa-enterprise.
import { expect, test } from '@playwright/test';
import { eap8021xSettings, emptyEap8021xForm, validateEap8021x } from '../src/lib/eap-8021x';

const CA = '-----BEGIN CERTIFICATE-----\nMIIB\n-----END CERTIFICATE-----\n';
const KEY = '-----BEGIN PRIVATE KEY-----\nMIIE\n-----END PRIVATE KEY-----\n';

test.describe('802.1X settings', () => {
	test('PEAP sends identity, outer identity, inner method and domain', () => {
		expect(
			eap8021xSettings({
				...emptyEap8021xForm(),
				identity: ' alice ',
				anonymousIdentity: 'anonymous',
				password: 'hunter2',
				domain: 'Radius.Example.com',
				caCert: CA,
				caCertName: 'ca.pem',
				clientCert: CA,
				privateKey: KEY
			})
		).toEqual({
			eap: 'peap',
			identity: 'alice',
			anonymous_identity: 'anonymous',
			password: 'hunter2',
			ca_cert: CA,
			client_cert: undefined,
			private_key: undefined,
			private_key_password: undefined,
			domain_suffix_match: 'radius.example.com',
			phase2: 'mschapv2'
		});
	});

	test('TLS sends the client certificate and key instead of a password', () => {
		const settings = eap8021xSettings({
			...emptyEap8021xForm(),
			eap: 'tls',
			identity: 'host.example.com',
			anonymousIdentity: 'ignored',
			password: 'ignored',
			clientCert: CA,
			privateKey: KEY,
			privateKeyPassword: 'keypass'
		});
		expect(JSON.parse(JSON.stringify(settings))).toEqual({
			eap: 'tls',
			identity: 'host.example.com',
			client_cert: CA,
			private_key: KEY,
			private_key_password: 'keypass'
		});
		expect(validateEap8021x(settings)).toEqual(JSON.parse(JSON.stringify(settings)));
	});

	test('validation keeps only Tetra fields and refuses old names', () => {
		expect(validateEap8021x({ eap: 'peap', username: 'alice', password: 'x' })).toBe(
			'Unknown 802.1X setting: username'
		);
		expect(validateEap8021x({ eap: 'peap', identity: 'a', domain: 'example.com' })).toBe(
			'Unknown 802.1X setting: domain'
		);
		expect(validateEap8021x({ type: 'wpa-enterprise', eap: 'peap', identity: 'a' })).toBe(
			'Unknown 802.1X setting: type'
		);
		expect(
			validateEap8021x({ type: 'wpa-enterprise', eap: 'peap', identity: 'a', password: 'p' }, [
				'type'
			])
		).toEqual({ eap: 'peap', identity: 'a', password: 'p' });
	});

	test('validation follows the EAP method', () => {
		expect(validateEap8021x({ eap: 'leap', identity: 'a', password: 'p' })).toBe(
			'Unsupported EAP method'
		);
		expect(validateEap8021x({ eap: 'peap', identity: ' ', password: 'p' })).toBe(
			'Identity is required'
		);
		expect(validateEap8021x({ eap: 'ttls', identity: 'a' })).toBe('Password is required');
		expect(validateEap8021x({ eap: 'tls', identity: 'a', client_cert: CA })).toBe(
			'EAP-TLS needs a client certificate and private key'
		);
		expect(validateEap8021x({ eap: 'peap', identity: 'a', password: 'p', phase2: 'eap-gtc' })).toBe(
			'Unsupported inner authentication method'
		);
	});

	test('certificates and keys must be PEM contents', () => {
		const base = { eap: 'peap', identity: 'a', password: 'p' };
		expect(validateEap8021x({ ...base, ca_cert: '/etc/pki/ca.pem' })).toBe(
			'CA certificate must be a PEM file'
		);
		expect(
			validateEap8021x({ eap: 'tls', identity: 'a', client_cert: CA, private_key: 'secret' })
		).toBe('Private key must be a PEM file');
		expect(validateEap8021x({ ...base, ca_cert: CA })).toEqual({ ...base, ca_cert: CA });
	});

	test('the server domain is a domain suffix', () => {
		const base = { eap: 'peap', identity: 'a', password: 'p' };
		expect(validateEap8021x({ ...base, domain_suffix_match: 'Example.COM' })).toEqual({
			...base,
			domain_suffix_match: 'example.com'
		});
		expect(validateEap8021x({ ...base, domain_suffix_match: 'not a domain' })).toBe(
			'Enter a domain such as example.com'
		);
	});
});
