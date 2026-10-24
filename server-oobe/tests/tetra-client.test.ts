// Unit tests for the Tetra client and its mock. They run in
// the Playwright runner (the repo's only test setup) without a browser.
import { expect, test } from '@playwright/test';
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { createServer, type Server } from 'node:http';
import { WebSocketServer } from 'ws';
import { SocketTetraClient, TETRA_PROTOCOL_VERSION, TetraError } from '../src/lib/server/tetra';
import { MockTetraClient } from '../src/lib/server/tetra-mock';

type Request = { type: string; id: string; token?: string; op: string; params: unknown };

/** A fake Tetra on a Unix socket that answers with `handler`. */
async function fakeTetra(
	handler: (request: Request) => unknown,
	hello: Record<string, unknown> = {}
): Promise<{ path: string; requests: Request[]; close: () => Promise<void> }> {
	const dir = await mkdtemp(join(tmpdir(), 'oobe-tetra-'));
	const path = join(dir, 'tetra.sock');
	const requests: Request[] = [];
	const server: Server = createServer();
	const wss = new WebSocketServer({ server });
	wss.on('connection', (socket) => {
		socket.send(
			JSON.stringify({
				type: 'hello',
				protocol_version: TETRA_PROTOCOL_VERSION,
				client: 'oobe',
				host_fingerprint: 'test',
				setup_required: true,
				passkeys_available: false,
				...hello
			})
		);
		socket.on('message', (data) => {
			const request = JSON.parse(data.toString()) as Request;
			requests.push(request);
			socket.send(
				JSON.stringify({ type: 'response', id: request.id, ...(handler(request) as object) })
			);
		});
	});
	await new Promise<void>((resolve) => server.listen(path, resolve));
	return {
		path,
		requests,
		close: async () => {
			for (const client of wss.clients) client.terminate();
			await new Promise((resolve) => server.close(resolve));
			await rm(dir, { recursive: true, force: true });
		}
	};
}

test.describe('SocketTetraClient', () => {
	test('redeems the setup code and sends the token with every request', async () => {
		const tetra = await fakeTetra((request) => {
			switch (request.op) {
				case 'setup.redeem':
					return {
						ok: true,
						result: {
							status: 'authenticated',
							token: 'tts_abc',
							session: { kind: 'setup', client: 'oobe', expires_at: 123 },
							admin_exists: false
						}
					};
				case 'session.info':
					return {
						ok: true,
						result: { session: { kind: 'setup', client: 'oobe', expires_at: 123 } }
					};
				case 'module.dispatch':
					return { ok: true, result: { hostname: 'box' } };
				default:
					return { ok: false, error: { code: 'unknown_operation', message: request.op } };
			}
		});
		try {
			const client = new SocketTetraClient(tetra.path);
			const session = await client.redeemSetupCode('ABCD-EFGH', '192.0.2.44');
			expect(session).toEqual({ token: 'tts_abc', expiresAt: 123, adminExists: false });
			expect(await client.sessionInfo('tts_abc')).toEqual({
				kind: 'setup',
				client: 'oobe',
				expiresAt: 123
			});
			expect(
				await client.dispatch('tts_abc', 'settings', 'set_hostname', { hostname: 'box' })
			).toEqual({ hostname: 'box' });

			expect(tetra.requests.map((r) => [r.op, r.token])).toEqual([
				['setup.redeem', undefined],
				['session.info', 'tts_abc'],
				['module.dispatch', 'tts_abc']
			]);
			expect(tetra.requests[0].params).toEqual({ code: 'ABCD-EFGH', remote_addr: '192.0.2.44' });
			expect(tetra.requests[2].params).toEqual({
				module: 'settings',
				action: 'set_hostname',
				payload: { hostname: 'box' }
			});
		} finally {
			await tetra.close();
		}
	});

	test('maps Tetra errors, including rate limits', async () => {
		const tetra = await fakeTetra(() => ({
			ok: false,
			error: { code: 'rate_limited', message: 'too many attempts', retry_after: 30 }
		}));
		try {
			const error = await new SocketTetraClient(tetra.path)
				.redeemSetupCode('nope')
				.catch((e: unknown) => e);
			expect(error).toBeInstanceOf(TetraError);
			expect((error as TetraError).code).toBe('rate_limited');
			expect((error as TetraError).retryAfter).toBe(30);
		} finally {
			await tetra.close();
		}
	});

	test('refuses an unknown protocol version', async () => {
		const tetra = await fakeTetra(() => ({ ok: true, result: {} }), {
			protocol_version: '1999-01-old'
		});
		try {
			const error = await new SocketTetraClient(tetra.path).setupStatus().catch((e: unknown) => e);
			expect((error as TetraError).code).toBe('unavailable');
		} finally {
			await tetra.close();
		}
	});

	test('reports an unreachable socket as unavailable', async () => {
		const error = await new SocketTetraClient('/nonexistent/tetra.sock')
			.setupStatus()
			.catch((e: unknown) => e);
		expect((error as TetraError).code).toBe('unavailable');
	});
});

test.describe('SocketTetraClient setup actions', () => {
	test('sends access.set and parses setup.complete', async () => {
		const tetra = await fakeTetra((request) =>
			request.op === 'access.set'
				? { ok: true, result: { passkeys_available: false, dashboard_url: 'https://192.0.2.10' } }
				: request.op === 'setup.complete'
					? {
							ok: true,
							result: { complete: true, dashboard_url: 'https://192.0.2.10', handoff_in_seconds: 3 }
						}
					: { ok: false, error: { code: 'unknown_operation', message: request.op } }
		);
		try {
			const client = new SocketTetraClient(tetra.path);
			expect(await client.setAccess('tts_x', { mode: 'ip', hostname: '192.0.2.10' })).toEqual({
				passkeysAvailable: false,
				dashboardUrl: 'https://192.0.2.10'
			});
			expect(await client.completeSetup('tts_x')).toEqual({
				dashboardUrl: 'https://192.0.2.10',
				handoffInSeconds: 3
			});
			expect(tetra.requests[0]).toMatchObject({
				op: 'access.set',
				token: 'tts_x',
				params: { mode: 'ip', hostname: '192.0.2.10' }
			});
		} finally {
			await tetra.close();
		}
	});
});

test.describe('MockTetraClient', () => {
	test('the setup session may only run allowed actions', async () => {
		const tetra = new MockTetraClient('ABCD-EFGH');
		const { token } = await tetra.redeemSetupCode('ABCD-EFGH');
		for (const [module, action] of [
			['services', 'enable'],
			['services', 'start'],
			['network', 'set_config'],
			['files', 'write']
		]) {
			await expect(tetra.dispatch(token, module, action)).rejects.toMatchObject({
				code: 'forbidden'
			});
		}
		expect(await tetra.dispatch(token, 'updates', 'set_policy', { policy: 'notify' })).toEqual({
			policy: 'notify'
		});
	});

	test('checks the code and the token', async () => {
		const tetra = new MockTetraClient('ABCD-EFGH');
		await expect(tetra.redeemSetupCode('ABCD-EFGX')).rejects.toMatchObject({
			code: 'invalid_credentials'
		});
		await expect(tetra.dispatch('tts_forged', 'network', 'interfaces')).rejects.toMatchObject({
			code: 'unauthenticated'
		});
		const { token } = await tetra.redeemSetupCode('abcd efgh', '192.0.2.44');
		expect(tetra.redeemAttempts).toEqual([
			{ remoteAddr: undefined, ok: false },
			{ remoteAddr: '192.0.2.44', ok: true }
		]);
		const { interfaces } = await tetra.dispatch(token, 'network', 'interfaces');
		expect((interfaces as { name: string }[]).map((entry) => entry.name)).toEqual([
			'enp1s0',
			'lo',
			'virbr0',
			'wlan0'
		]);
		expect(tetra.calls).toEqual([{ module: 'network', action: 'interfaces', payload: {} }]);
	});

	test('network.configure follows Tetra: unknown fields, safe apply, confirm', async () => {
		const tetra = new MockTetraClient('ABCD-EFGH');
		const { token } = await tetra.redeemSetupCode('ABCD-EFGH');
		tetra.useFixtureHost(token, 'ipv6-only');
		await expect(
			tetra.dispatch(token, 'network', 'configure', {
				interface: 'enp1s0',
				ipv4: { method: 'dhcp' }
			})
		).rejects.toMatchObject({ code: 'bad_request' });
		await expect(
			tetra.dispatch(token, 'network', 'configure', {
				interface: 'enp1s0',
				ipv6: { method: 'auto', enabled: true }
			})
		).rejects.toMatchObject({ code: 'bad_request' });
		await expect(
			tetra.dispatch(token, 'network', 'configure', {
				interface: 'enp1s0',
				ipv4: { method: 'disabled' },
				ipv6: { method: 'link-local' }
			})
		).rejects.toMatchObject({ code: 'bad_request' });

		// Keeping the current address needs no confirmation.
		expect(
			await tetra.dispatch(token, 'network', 'configure', {
				interface: 'enp1s0',
				ipv4: { method: 'disabled' },
				ipv6: { method: 'manual', addresses: [{ address: '2001:db8:10::2', prefix_length: 64 }] }
			})
		).toMatchObject({ applied: true, confirmation_required: false });

		const moved = await tetra.dispatch(token, 'network', 'configure', {
			interface: 'enp1s0',
			ipv4: { method: 'disabled' },
			ipv6: { method: 'manual', addresses: [{ address: '2001:db8:10::99', prefix_length: 64 }] }
		});
		expect(moved).toMatchObject({ confirmation_required: true, rollback_in_seconds: 60 });
		await expect(
			tetra.dispatch(token, 'network', 'confirm', { token: 'wrong' })
		).rejects.toMatchObject({ code: 'action_failed' });
		expect(
			await tetra.dispatch(token, 'network', 'confirm', { token: moved.confirmation_token })
		).toMatchObject({ confirmed: true });
		expect(JSON.stringify(tetra.interfaces(token))).toContain('2001:db8:10::99');
	});

	test('network actions follow the interface type', async () => {
		const tetra = new MockTetraClient('ABCD-EFGH');
		const { token } = await tetra.redeemSetupCode('ABCD-EFGH');
		const { interfaces } = await tetra.dispatch(token, 'network', 'interfaces');
		expect(
			(interfaces as { name: string; type: string }[]).map(({ name, type }) => [name, type])
		).toEqual([
			['enp1s0', 'ethernet'],
			['lo', 'other'],
			['virbr0', 'other'],
			['wlan0', 'wifi']
		]);
		for (const name of ['wlan0', 'virbr0']) {
			await expect(
				tetra.dispatch(token, 'network', 'configure', { interface: name })
			).rejects.toMatchObject({ code: 'action_failed' });
		}
		await expect(
			tetra.dispatch(token, 'network', 'scan_wifi', { interface: 'enp1s0' })
		).rejects.toMatchObject({ code: 'action_failed' });
		expect(
			await tetra.dispatch(token, 'network', 'scan_wifi', { interface: 'wlan0' })
		).toMatchObject({ networks: expect.any(Array) });
	});

	test('setup.complete needs an administrator and ends setup sessions', async () => {
		const tetra = new MockTetraClient('ABCD-EFGH');
		const { token } = await tetra.redeemSetupCode('ABCD-EFGH');
		await expect(tetra.completeSetup(token)).rejects.toMatchObject({ code: 'setup_incomplete' });

		await tetra.setAccess(token, {
			mode: 'domain',
			hostname: 'box.example.com',
			challenge: 'http-01'
		});
		await tetra.dispatch(token, 'users', 'add_admin', { name: 'a' });
		expect(await tetra.completeSetup(token)).toEqual({
			dashboardUrl: 'https://box.example.com',
			handoffInSeconds: 5
		});
		await expect(tetra.sessionInfo(token)).rejects.toMatchObject({ code: 'unauthenticated' });
		await expect(tetra.redeemSetupCode('ABCD-EFGH')).rejects.toMatchObject({
			code: 'setup_complete'
		});
	});
});
