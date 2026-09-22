<script lang="ts">
	import Shell from '$lib/components/Shell.svelte';
	import WelcomeStep from '$lib/components/steps/01-welcome.svelte';
	import DeviceNameStep from '$lib/components/steps/03-device-name.svelte';
	import UserStep from '$lib/components/steps/04-user.svelte';
	import PasswordStep from '$lib/components/steps/05-password.svelte';
	import InternetStep from '$lib/components/steps/06-internet.svelte';
	import TweaksStep from '$lib/components/steps/07-tweaks.svelte';
	import SshKeyStep from '$lib/components/steps/08-ssh-key.svelte';
	import DashboardStep from '$lib/components/steps/09-dashboard.svelte';
	import CompleteStep from '$lib/components/steps/10-complete.svelte';
	import { api } from '$lib/oobe-api';
	import {
		fixtureState,
		fixtureSteps,
		stepIndex,
		type StepId,
		type OobeState
	} from '$lib/oobe-state';
	import { onMount } from 'svelte';
	import { detectLocale } from '$lib/i18n.svelte';
	import { detectKeyboardLayout } from '$lib/keyboard-detect';

	let oobeState = $state<OobeState>(fixtureState);
	let loaded = $state(false);
	let errorMessage = $state('');
	let operationLoading = $state(false);
	let password = $state('');
	let redirecting = $state(false);
	let retryStep = $state<StepId | null>(null);
	let networkConfig = $state<Record<string, unknown>>({});

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
		withTimeout(api.getState(), 5000)
			.then((s) => {
				oobeState = s;
				loaded = true;
				if (s.activeStep === 'internet') void refreshNetworkInterfaces();
			})
			.catch((err) => {
				console.error('Failed to load OOBE state:', err);
				errorMessage = 'Unable to load setup state. Using defaults.';
				loaded = true;
			});
	});

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
		const res = await fetch('/api/oobe/operations', {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ step: selectedStep, operation: 'state.patch', payload })
		});
		if (!res.ok) throw new Error(`HTTP ${res.status}`);
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

	async function markCompleted() {
		try {
			await callStatePatch({ completed: true });
			oobeState = { ...oobeState, completed: true };
		} catch (err) {
			console.error('Failed to mark completed:', err);
			oobeState = { ...oobeState, completed: true };
		}
	}

	async function onContinue() {
		if (operationLoading) return;
		errorMessage = '';

		const step = selectedStep;

		// Last step: finish or hand off to local dashboard.
		if (currentIndex >= fixtureSteps.length - 1) {
			operationLoading = true;
			try {
				await markCompleted();

				await callOperation('dashboard.install');
				await callOperation('dashboard.handoff');
				redirecting = true;
				await new Promise((resolve) => setTimeout(resolve, 4000));
				window.location.href = '/';
			} catch (err) {
				errorMessage = String(err);
				redirecting = false;
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
			if (step === 'tweaks') {
				const tweaks = oobeState.tweaks ?? { ssh: true, firewall: true, automaticUpdates: true };
				await callOperation('tweaks.apply', tweaks);
				await callStatePatch({ tweaks });
			}
			if (step === 'ssh-key' && oobeState.sshPublicKey) {
				await callOperation('ssh-key.apply', { publicKey: oobeState.sshPublicKey });
			}
			if (step === 'dashboard') {
				await callStatePatch({
					dashboardDomain: oobeState.dashboardDomain,
					dashboardPort: oobeState.dashboardPort
				});
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
	{#if redirecting}
		<div class="redirect-overlay">
			<p class="redirect-title">Starting dashboard…</p>
			<p class="redirect-sub">You will be redirected automatically.</p>
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
				<WelcomeStep {onBack} {onContinue} />
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
			{:else if selectedStep === 'tweaks'}
				<TweaksStep bind:settings={oobeState.tweaks} {onBack} {onContinue} />
			{:else if selectedStep === 'ssh-key'}
				<SshKeyStep bind:value={oobeState.sshPublicKey} {onBack} {onContinue} />
			{:else if selectedStep === 'dashboard'}
				<DashboardStep
					bind:dashboardDomain={oobeState.dashboardDomain}
					bind:dashboardPort={oobeState.dashboardPort}
					{onBack}
					{onContinue}
				/>
			{:else}
				<CompleteStep
					{onBack}
					{onContinue}
					{onReboot}
					{onShutdown}
					hostname={oobeState.hostname}
					administrator={oobeState.administrator}
					dashboardDomain={oobeState.dashboardDomain}
					dashboardPort={oobeState.dashboardPort}
				/>
			{/if}
		</Shell>
	{/if}
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
	.redirect-overlay {
		position: fixed;
		inset: 0;
		z-index: 200;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 0.5rem;
		background: var(--background);
		color: var(--foreground);
	}
	.redirect-title {
		font-size: 1.25rem;
		font-weight: 600;
		margin: 0;
	}
	.redirect-sub {
		color: var(--muted-foreground);
		font-size: 0.9rem;
		margin: 0;
	}
</style>
