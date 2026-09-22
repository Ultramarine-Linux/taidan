import { spawn } from 'node:child_process';
import { env } from 'node:process';

export interface TetraCommand {
	id: string;
	module: string;
	action: string;
	payload?: Record<string, unknown>;
	signature?: string;
}

export interface TetraResponse {
	id: string;
	ok: boolean;
	payload?: Record<string, unknown>;
	error?: string;
}

function tetraPath(): string {
	return env.TETRA_PATH || 'tetra';
}

export function dispatchTetra(command: TetraCommand): Promise<TetraResponse> {
	return new Promise((resolve, reject) => {
		const child = spawn(tetraPath(), ['agent-dispatch'], {
			stdio: ['pipe', 'pipe', 'pipe']
		});

		let stdout = '';
		let stderr = '';

		child.stdout.on('data', (data) => {
			stdout += data.toString();
		});

		child.stderr.on('data', (data) => {
			stderr += data.toString();
		});

		child.on('error', (err) => {
			reject(new Error(`Failed to spawn tetra: ${err.message}`));
		});

		child.on('close', (code) => {
			if (code !== 0) {
				reject(new Error(`tetra exited with code ${code}: ${stderr || stdout}`));
				return;
			}
			try {
				const response = JSON.parse(stdout.trim()) as TetraResponse;
				resolve(response);
			} catch {
				reject(new Error(`Invalid JSON from tetra: ${stdout.trim()}`));
			}
		});

		child.stdin.write(JSON.stringify(command));
		child.stdin.end();
	});
}

export async function tetraCapabilities(): Promise<TetraResponse> {
	return dispatchTetra({
		id: 'oobe-capabilities',
		module: 'agent',
		action: 'capabilities',
		payload: {}
	});
}

export async function tetraNetworkInterfaces(): Promise<TetraResponse> {
	return dispatchTetra({
		id: 'oobe-network-interfaces',
		module: 'network',
		action: 'interfaces',
		payload: {}
	});
}

export async function tetraNetworkScanWifi(interfaceName: string): Promise<TetraResponse> {
	return dispatchTetra({
		id: 'oobe-network-scan-wifi',
		module: 'network',
		action: 'scan_wifi',
		payload: { interface: interfaceName }
	});
}

export async function tetraNetworkConnectWifi(
	interfaceName: string,
	ssid: string,
	security: Record<string, unknown>
): Promise<TetraResponse> {
	return dispatchTetra({
		id: 'oobe-network-connect-wifi',
		module: 'network',
		action: 'connect_wifi',
		payload: { interface: interfaceName, ssid, security }
	});
}

export async function tetraNetworkSetConfig(
	config: Record<string, unknown>
): Promise<TetraResponse> {
	return dispatchTetra({
		id: 'oobe-network-set-config',
		module: 'network',
		action: 'set_config',
		payload: config
	});
}

export async function tetraNetworkStatus(): Promise<TetraResponse> {
	return dispatchTetra({
		id: 'oobe-network-status',
		module: 'network',
		action: 'status',
		payload: {}
	});
}

export async function tetraSetHostname(hostname: string): Promise<TetraResponse> {
	return dispatchTetra({
		id: 'oobe-set-hostname',
		module: 'settings',
		action: 'set_hostname',
		payload: { hostname }
	});
}

export async function tetraCreateUser(
	name: string,
	shell?: string,
	home?: string
): Promise<TetraResponse> {
	return dispatchTetra({
		id: 'oobe-create-user',
		module: 'users',
		action: 'create',
		payload: { name, shell, home }
	});
}

export async function tetraSetPassword(name: string, password: string): Promise<TetraResponse> {
	return dispatchTetra({
		id: 'oobe-set-password',
		module: 'users',
		action: 'set_password',
		payload: { name, password }
	});
}

export async function tetraServiceStatus(service: string): Promise<TetraResponse> {
	return dispatchTetra({
		id: 'oobe-service-status',
		module: 'services',
		action: 'status',
		payload: { service }
	});
}

export async function tetraServiceStart(service: string): Promise<TetraResponse> {
	return dispatchTetra({
		id: 'oobe-service-start',
		module: 'services',
		action: 'start',
		payload: { service }
	});
}

export async function tetraServiceEnable(service: string): Promise<TetraResponse> {
	return dispatchTetra({
		id: 'oobe-service-enable',
		module: 'services',
		action: 'enable',
		payload: { service }
	});
}
