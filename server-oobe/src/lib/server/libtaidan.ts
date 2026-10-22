import { spawn } from 'node:child_process';
import type { StepId } from '$lib/oobe-state';

const FIXTURE_MODE = process.env.OOBE_FIXTURE_MODE === 'true';

// Every operation is validated by libtaidan (`tdnx server-oobe`) before it
// runs. There is no opt-out: if tdnx is missing or rejects the request, the
// operation fails. Fixture mode (development and tests only) has no tdnx.
export function validateWithLibtaidan(
	step: StepId,
	operation: string,
	payload: Record<string, unknown>
): Promise<void> {
	if (FIXTURE_MODE) return Promise.resolve();
	return new Promise((resolve, reject) => {
		const child = spawn(process.env.TDNX_PATH || 'tdnx', ['server-oobe'], {
			stdio: ['pipe', 'pipe', 'pipe']
		});
		let stdout = '';
		let stderr = '';
		child.stdout.on('data', (data) => (stdout += data.toString()));
		child.stderr.on('data', (data) => (stderr += data.toString()));
		child.on('error', (e) => reject(new Error(`libtaidan validation unavailable: ${e.message}`)));
		child.on('close', (code) => {
			let response: { ok?: boolean; error?: string } | undefined;
			try {
				response = JSON.parse(stdout.trim()) as { ok?: boolean; error?: string };
			} catch {
				response = undefined;
			}
			if (code === 0 && response?.ok === true) {
				resolve();
				return;
			}
			reject(
				new Error(
					response?.error ||
						stderr.trim() ||
						(code === 0
							? 'Invalid response from libtaidan backend'
							: `libtaidan exited with code ${code}`)
				)
			);
		});
		child.stdin.end(JSON.stringify({ step, operation, payload }));
	});
}
