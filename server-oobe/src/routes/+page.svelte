<script lang="ts">
	import Shell from '$lib/components/Shell.svelte';
	import WelcomeStep from '$lib/components/steps/01-welcome.svelte';
	import DeviceNameStep from '$lib/components/steps/03-device-name.svelte';
	import UserStep from '$lib/components/steps/04-user.svelte';
	import PasswordStep from '$lib/components/steps/05-password.svelte';
	import InternetStep from '$lib/components/steps/06-internet.svelte';
	import UpdatesStep from '$lib/components/steps/07-updates.svelte';
	import RemoteAccessStep from '$lib/components/steps/08-remote-access.svelte';
	import SshKeyStep from '$lib/components/steps/08-ssh-key.svelte';
	import DashboardStep from '$lib/components/steps/09-dashboard.svelte';
	import CompleteStep from '$lib/components/steps/10-complete.svelte';
	import { api, SetupSessionRequiredError } from '$lib/oobe-api';
	import {
		fixtureState,
		fixtureSteps,
		stepIndex,
		type AccessForm,
		type OobeState,
		type RemoteAccessForm,
		type SetupCompleteResult,
		type StepId
	} from '$lib/oobe-state';
	import { onMount } from 'svelte';
	import { detectLocale, t } from '$lib/i18n.svelte';
	import { detectKeyboardLayout } from '$lib/keyboard-detect';

	let oobeState = $state<OobeState>(fixtureState);
	let loaded = $state(false);
	let errorMessage = $state('');
	let operationLoading = $state(false);
	let password = $state('');
	let retryStep = $state<StepId | null>(null);
	let networkConfig = $state<Record<string, unknown>>({});
	let ipAddresses = $state<string[]>([]);
	let authenticated = $state(false);
	let setupCode = $state('');
	let setupCodeError = $state('');
	let finished = $state<SetupCompleteResult | null>(null);
	// Secrets (auth keys, tunnel tokens, DNS credentials) live only here, are
	// sent to Tetra once, and are cleared afterwards. They are never persisted.
	let remoteAccessForm = $state<RemoteAccessForm>({
		tailscale: false,
		tailscaleAuthKey: '',
		tailscaleServe: true,
		tailscaleFunnel: false,
		funnelConfirmed: false,
		cloudflared: false,
		cloudflaredToken: ''
	});
	let accessForm = $state<AccessForm>({
		mode: 'domain',
		hostname: '',
		challenge: 'http-01',
		dnsProvider: '',
		dnsCredentials: {}
	});

	let selectedStep = $derived(oobeState.activeStep);
	let currentIndex = $derived(stepIndex(selectedStep));

	function withTimeout<T>(promise: Promise<T>, ms: number): Promise<T> {
		return new Promise((resolve, reject) => {
			const timer = setTimeout(() => reject(new Error('Timed out loading state')), ms);
			promise
				.then((v) => {
					clearTimeout(timer);
					resolve(v);
				})
				.catch((err) => {
					clearTimeout(timer);
					reject(err);
				});
		});
	}

	onMount(() => {
		detectLocale();
		oobeState.keyboardLayout = detectKeyboardLayout();
		fetch('/api/oobe/network')
			.then((res) => (res.ok ? res.json() : { addresses: [] }))
			.then((data: { addresses?: string[] }) => (ipAddresses = data.addresses ?? []))
			.catch(() => (ipAddresses = []));
		withTimeout(api.getSession(), 5000)
			.then(async (session) => {
				if (session.error === 'unavailable') errorMessage = t('setup-code-unavailable');
				else if (!session.setupRequired) errorMessage = t('setup-code-complete');
				authenticated = session.authenticated;
				if (authenticated) await loadState();
				loaded = true;
			})
			.catch((err) => {
				console.error('Failed to check setup session:', err);
				errorMessage = t('setup-code-unavailable');
				loaded = true;
			});
	});

	async function loadState() {
		try {
			const s = await withTimeout(api.getState(), 5000);
			oobeState = s;
			accessForm = {
				...accessForm,
				mode: s.access.mode,
				hostname: s.access.hostname,
				challenge: s.access.challenge,
				dnsProvider: s.access.dnsProvider ?? ''
			};
			if (s.activeStep === 'internet') void refreshNetworkInterfaces();
		} catch (err) {
			if (err instanceof SetupSessionRequiredError) return requireSetupCode();
			console.error('Failed to load OOBE state:', err);
			errorMessage = 'Unable to load setup state. Using defaults.';
		}
	}

	/** The setup session is missing or expired: ask for the code again on Welcome. */
	function requireSetupCode() {
		authenticated = false;
		retryStep = null;
		setupCodeError = t('setup-code-expired');
		oobeState = { ...oobeState, activeStep: 'welcome' };
	}

	async function redeemSetupCode(): Promise<boolean> {
		setupCodeError = '';
		const session = await api.redeemSetupCode(setupCode);
		if (session.authenticated) {
			authenticated = true;
			setupCode = '';
			return true;
		}
		setupCodeError =
			session.error === 'rate_limited'
				? t('setup-code-rate-limited', { seconds: session.retryAfter ?? 60 })
				: session.error === 'setup_complete'
					? t('setup-code-complete')
					: session.error === 'unavailable'
						? t('setup-code-unavailable')
						: t('setup-code-invalid');
		return false;
	}

	async function refreshNetworkInterfaces() {
		try {
			const result = await api.startOperation('internet', 'network.interfaces');
			if (result.status === 'succeeded' && result.message) {
				oobeState.networkInterfaces = JSON.parse(result.message);
			}
		} catch (err) {
			console.error('Failed to load network interfaces:', err);
		}
	}

	async function saveActiveStep(step: StepId) {
		if (!authenticated && step !== 'welcome') return;
		try {
			await callStatePatch({ activeStep: step });
			oobeState = { ...oobeState, activeStep: step };
			if (step === 'internet') void refreshNetworkInterfaces();
		} catch (err) {
			console.error('Failed to save active step:', err);
			oobeState = { ...oobeState, activeStep: step };
		}
	}

	async function scanWifi(interfaceName: string) {
		const result = await api.startOperation('internet', 'network.scan_wifi', {
			interface: interfaceName
		});
		if (result.status === 'failed') throw new Error(result.message || 'Wi-Fi scan failed');
		return result.message
			? (JSON.parse(result.message) as import('$lib/oobe-state').WifiNetwork[])
			: [];
	}

	async function connectWifi(
		interfaceName: string,
		ssid: string,
		security: import('$lib/oobe-state').WifiSecurityProfile
	) {
		const result = await api.startOperation('internet', 'network.connect_wifi', {
			interface: interfaceName,
			ssid,
			security
		});
		if (result.status === 'failed') throw new Error(result.message || 'Wi-Fi connection failed');
	}

	async function callOperation(operation: string, payload?: Record<string, unknown>) {
		const res = await api.startOperation(selectedStep, operation, payload);
		if (res.status === 'failed') {
			throw new Error(res.message || 'Operation failed');
		}
		return res;
	}

	async function callStatePatch(payload: Record<string, unknown>) {
		const res = await api.startOperation(selectedStep, 'state.patch', payload);
		if (res.status === 'failed') throw new Error(res.message || 'Failed to save setup state');
	}

	function onBack() {
		if (currentIndex > 0) {
			saveActiveStep(fixtureSteps[currentIndex - 1].id);
		}
	}

	async function patchStepStatus(step: StepId, status: 'complete' | 'failed') {
		try {
			await callStatePatch({ steps: [{ id: step, status }] });
			oobeState = {
				...oobeState,
				steps: oobeState.steps.map((current) =>
					current.id === step ? { ...current, status } : current
				)
			};
		} catch (err) {
			console.error('Failed to patch step status:', err);
			const updatedSteps = oobeState.steps.map((s) => (s.id === step ? { ...s, status } : s));
			oobeState = { ...oobeState, steps: updatedSteps };
		}
	}

	async function onContinue() {
		if (operationLoading) return;
		errorMessage = '';

		const step = selectedStep;

		// Nothing else can run until Tetra has accepted the setup code.
		if (step === 'welcome' && !authenticated) {
			operationLoading = true;
			try {
				if (!(await redeemSetupCode())) return;
				await loadState();
				// Resume where a previous session left off.
				if (oobeState.activeStep !== 'welcome') return;
			} catch (err) {
				errorMessage = String(err);
				return;
			} finally {
				operationLoading = false;
			}
		}

		// Last step. setup.complete ends this setup session, disables the code,
		// starts the dashboard and stops this OOBE a few seconds later, so it is
		// the final call and the final screen comes from its response.
		if (currentIndex >= fixtureSteps.length - 1) {
			operationLoading = true;
			try {
				const result = await callOperation('setup.complete');
				finished = JSON.parse(result.message ?? '{}') as SetupCompleteResult;
				authenticated = false;
			} catch (err) {
				if (err instanceof SetupSessionRequiredError) requireSetupCode();
				else errorMessage = String(err);
			} finally {
				operationLoading = false;
			}
			return;
		}

		operationLoading = true;
		try {
			if (step === 'devicename') {
				if (oobeState.hostname) {
					await callOperation('hostname.apply', { hostname: oobeState.hostname });
				}
				if (oobeState.keyboardLayout) {
					await callOperation('keyboard.apply', {
						layout: oobeState.keyboardLayout,
						variant: oobeState.keyboardVariant
					});
				}
			}
			if (step === 'whoareyou') {
				if (oobeState.administrator) {
					await callOperation('user.create', { name: oobeState.administrator });
				}
			}
			if (step === 'password') {
				if (password && oobeState.administrator) {
					await callOperation('password.set', { name: oobeState.administrator, password });
					password = '';
				}
			}
			if (step === 'internet') {
				const interfaces = await callOperation('network.interfaces');
				if (interfaces.message) {
					try {
						oobeState.networkInterfaces = JSON.parse(interfaces.message);
					} catch {
						// The operation succeeded; an older server may not return interface details.
					}
				}
				if (Object.keys(networkConfig).length > 0) {
					await callOperation('network.set_config', networkConfig);
					await callStatePatch({ networkInterfaces: oobeState.networkInterfaces });
				}
				await callOperation('network.check');
			}
			if (step === 'updates') {
				await callOperation('updates.set_policy', { policy: oobeState.updatePolicy });
			}
			if (step === 'remote-access') {
				const form = remoteAccessForm;
				if (form.tailscale) {
					await callOperation('remote_access.tailscale_up', {
						auth_key: form.tailscaleAuthKey,
						serve: form.tailscaleServe,
						funnel: form.tailscaleFunnel,
						confirmFunnel: form.funnelConfirmed
					});
					form.tailscaleAuthKey = '';
					oobeState.remoteAccess = {
						...oobeState.remoteAccess,
						tailscale: true,
						tailscaleServe: form.tailscaleServe,
						tailscaleFunnel: form.tailscaleFunnel
					};
				}
				if (form.cloudflared) {
					await callOperation('remote_access.cloudflared_set_token', {
						token: form.cloudflaredToken
					});
					form.cloudflaredToken = '';
					oobeState.remoteAccess = { ...oobeState.remoteAccess, cloudflared: true };
				}
			}
			if (step === 'ssh-key' && oobeState.sshPublicKey) {
				await callOperation('ssh-key.apply', { publicKey: oobeState.sshPublicKey });
			}
			if (step === 'dashboard') {
				const form = accessForm;
				const hostname = form.hostname.trim();
				const payload =
					form.mode === 'ip'
						? { mode: 'ip', hostname }
						: form.challenge === 'http-01'
							? { mode: 'domain', hostname, challenge: 'http-01' }
							: {
									mode: 'domain',
									hostname,
									challenge: 'dns-01',
									dns_provider: form.dnsProvider,
									dns_credentials: form.dnsCredentials
								};
				const result = await callOperation('access.set', payload);
				const applied = JSON.parse(result.message ?? '{}') as { passkeysAvailable?: boolean };
				form.dnsCredentials = {};
				oobeState.access = {
					mode: form.mode,
					hostname,
					challenge: form.challenge,
					dnsProvider:
						form.mode === 'domain' && form.challenge === 'dns-01' ? form.dnsProvider : undefined,
					applied: true,
					passkeysAvailable: applied.passkeysAvailable
				};
			}

			await api.completeStep(step);
			retryStep = null;
			// Update local step status so sidebar shows the checkmark immediately
			oobeState.steps = oobeState.steps.map((s) =>
				s.id === step ? { ...s, status: 'complete' as import('$lib/oobe-state').StepStatus } : s
			);
			const nextStep = fixtureSteps[currentIndex + 1]?.id;
			if (nextStep) await saveActiveStep(nextStep);
		} catch (err) {
			if (err instanceof SetupSessionRequiredError) {
				requireSetupCode();
				return;
			}
			errorMessage = String(err);
			retryStep = step;
			await patchStepStatus(step, 'failed');
		} finally {
			operationLoading = false;
		}
	}

	async function onReboot() {
		operationLoading = true;
		try {
			await callOperation('system.reboot');
		} catch (err) {
			errorMessage = String(err);
		} finally {
			operationLoading = false;
		}
	}

	async function onShutdown() {
		operationLoading = true;
		try {
			await callOperation('system.poweroff');
		} catch (err) {
			errorMessage = String(err);
		} finally {
			operationLoading = false;
		}
	}
</script>

<svelte:head>
	<title>Server setup / Ultramarine Server</title>
	<meta name="description" content="Local first-run setup for an Ultramarine Server host." />
</svelte:head>

{#if !loaded}
	<div class="loading-screen">
		<p>Loading setup state…</p>
	</div>
{:else}
	{#if errorMessage}
		<div class="global-error" role="alert">
			<p>{errorMessage}</p>
			{#if retryStep}
				<button
					type="button"
					onclick={() => {
						errorMessage = '';
						void onContinue();
					}}
				>
					Retry
				</button>
			{/if}
			<button
				type="button"
				onclick={() => {
					errorMessage = '';
					retryStep = null;
				}}>Dismiss</button
			>
		</div>
	{/if}

	<Shell steps={oobeState.steps} {selectedStep} onSelectStep={saveActiveStep}>
		{#if selectedStep === 'welcome'}
			<WelcomeStep
				{onBack}
				{onContinue}
				{ipAddresses}
				needsSetupCode={!authenticated}
				bind:setupCode
				{setupCodeError}
			/>
		{:else if selectedStep === 'devicename'}
			<DeviceNameStep bind:hostname={oobeState.hostname} {onBack} {onContinue} />
		{:else if selectedStep === 'whoareyou'}
			<UserStep bind:username={oobeState.administrator} {onBack} {onContinue} />
		{:else if selectedStep === 'password'}
			<PasswordStep bind:password {onBack} {onContinue} />
		{:else if selectedStep === 'internet'}
			<InternetStep
				bind:interfaces={oobeState.networkInterfaces}
				bind:networkConfig
				onWifiScan={scanWifi}
				onWifiConnect={connectWifi}
				{onBack}
				{onContinue}
			/>
		{:else if selectedStep === 'updates'}
			<UpdatesStep bind:policy={oobeState.updatePolicy} {onBack} {onContinue} />
		{:else if selectedStep === 'remote-access'}
			<RemoteAccessStep bind:form={remoteAccessForm} {onBack} {onContinue} />
		{:else if selectedStep === 'ssh-key'}
			<SshKeyStep bind:value={oobeState.sshPublicKey} {onBack} {onContinue} />
		{:else if selectedStep === 'dashboard'}
			<DashboardStep bind:form={accessForm} {ipAddresses} {onBack} {onContinue} />
		{:else}
			<CompleteStep
				{onBack}
				{onContinue}
				{onReboot}
				{onShutdown}
				hostname={oobeState.hostname}
				administrator={oobeState.administrator}
				access={oobeState.access}
				updatePolicy={oobeState.updatePolicy}
				remoteAccess={oobeState.remoteAccess}
				{finished}
			/>
		{/if}
	</Shell>
{/if}

<style>
	.loading-screen {
		display: flex;
		align-items: center;
		justify-content: center;
		height: 100vh;
	}
	.global-error {
		position: fixed;
		top: 1rem;
		left: 50%;
		transform: translateX(-50%);
		background: #b91c1c;
		color: white;
		padding: 0.75rem 1rem;
		border-radius: 0.5rem;
		z-index: 100;
		display: flex;
		gap: 0.75rem;
		align-items: center;
	}
	.global-error button {
		background: white;
		color: #b91c1c;
		border: none;
		padding: 0.25rem 0.5rem;
		border-radius: 0.25rem;
		cursor: pointer;
	}
</style>
