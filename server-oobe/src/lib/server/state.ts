import { readFile, writeFile, mkdir, access } from 'node:fs/promises';
import { constants } from 'node:fs';
import { dirname } from 'node:path';
import { tmpdir } from 'node:os';
import {
	defaultAccess,
	defaultRemoteAccess,
	fixtureSteps,
	type OobeState,
	type StepId
} from '$lib/oobe-state';

/**
 * Port browsers use to reach the OOBE. Tetra serves it through Caddy with
 * `tls internal`; Node itself listens only on 127.0.0.1:6624.
 */
export const DEFAULT_OOBE_PORT = 6623;

const DEFAULT_STATE_PATH = '/var/lib/ultramarine-server-oobe/state.json';
const FALLBACK_STATE_PATH = `${tmpdir()}/ultramarine-server-oobe/state.json`;

function statePath(): string {
	return process.env.OOBE_STATE_PATH || DEFAULT_STATE_PATH;
}

function fallbackStatePath(): string {
	return FALLBACK_STATE_PATH;
}

const knownStepIds = new Set(fixtureSteps.map((s) => s.id));

function normalizeState(raw: unknown): OobeState {
	const base = defaultState();
	if (!raw || typeof raw !== 'object') return base;
	const partial = raw as Partial<OobeState>;

	const steps = fixtureSteps.map((fixture) => {
		const persisted = partial.steps?.find((s) => s.id === fixture.id);
		return persisted ? { ...fixture, status: persisted.status } : fixture;
	});

	let activeStep = partial.activeStep ?? base.activeStep;
	if (!knownStepIds.has(activeStep as StepId)) {
		activeStep = 'welcome';
	}

	return {
		version: 1,
		completed: partial.completed ?? false,
		activeStep: activeStep as StepId,
		steps,
		hostname: partial.hostname ?? '',
		administrator: partial.administrator ?? '',
		keyboardLayout: partial.keyboardLayout,
		keyboardVariant: partial.keyboardVariant,
		updatePolicy: partial.updatePolicy === 'notify' ? 'notify' : 'automatic',
		sshPublicKey: partial.sshPublicKey ?? '',
		networkInterfaces: partial.networkInterfaces ?? [],
		access: { ...defaultAccess, ...partial.access },
		remoteAccess: { ...defaultRemoteAccess, ...partial.remoteAccess }
	};
}

function defaultState(): OobeState {
	return {
		version: 1,
		completed: false,
		activeStep: 'welcome',
		steps: fixtureSteps.map((f) => ({ ...f })),
		hostname: '',
		administrator: '',
		keyboardVariant: undefined,
		updatePolicy: 'automatic',
		sshPublicKey: '',
		networkInterfaces: [],
		access: { ...defaultAccess },
		remoteAccess: { ...defaultRemoteAccess }
	};
}

export async function loadState(): Promise<OobeState> {
	async function tryLoad(path: string): Promise<OobeState | undefined> {
		try {
			await access(path, constants.R_OK);
			const raw = await readFile(path, 'utf-8');
			return normalizeState(JSON.parse(raw));
		} catch {
			return undefined;
		}
	}
	return (await tryLoad(statePath())) ?? (await tryLoad(fallbackStatePath())) ?? defaultState();
}

const FIXTURE_MODE = process.env.OOBE_FIXTURE_MODE === 'true';

export async function saveState(state: OobeState): Promise<void> {
	async function trySave(path: string): Promise<boolean> {
		try {
			await mkdir(dirname(path), { recursive: true });
			const tmp = `${path}.tmp`;
			await writeFile(tmp, JSON.stringify(state, null, 2) + '\n', 'utf-8');
			await writeFile(path, await readFile(tmp, 'utf-8'), 'utf-8');
			return true;
		} catch {
			return false;
		}
	}
	if (!(await trySave(statePath()))) {
		if (!(await trySave(fallbackStatePath()))) {
			if (FIXTURE_MODE) {
				console.warn('Unable to save OOBE state in fixture mode; continuing without persistence');
				return;
			}
			throw new Error('Unable to save OOBE state to primary or fallback path');
		}
	}
}
