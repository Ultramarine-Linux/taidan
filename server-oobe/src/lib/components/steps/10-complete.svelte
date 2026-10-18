<script lang="ts">
	import StepLayout from '$lib/components/StepLayout.svelte';
	import { t } from '$lib/i18n.svelte';
	import type {
		AccessState,
		RemoteAccessState,
		SetupCompleteResult,
		UpdatePolicy
	} from '$lib/oobe-state';
	let {
		onBack,
		onContinue,
		onReboot,
		onShutdown,
		hostname = 'ultramarine-server',
		administrator = 'Not configured',
		access,
		updatePolicy,
		remoteAccess,
		finished = null
	}: {
		onBack: () => void;
		onContinue: () => void;
		onReboot: () => void;
		onShutdown: () => void;
		hostname?: string;
		administrator?: string;
		access: AccessState;
		updatePolicy: UpdatePolicy;
		remoteAccess: RemoteAccessState;
		/** The setup.complete response, once setup has finished. */
		finished?: SetupCompleteResult | null;
	} = $props();

	let address = $derived(
		access.hostname
			? `https://${access.hostname.includes(':') ? `[${access.hostname}]` : access.hostname}`
			: ''
	);
	let remote = $derived(
		[
			remoteAccess.tailscale
				? remoteAccess.tailscaleFunnel
					? t('complete-remote-tailscale-funnel')
					: t('complete-remote-tailscale')
				: '',
			remoteAccess.cloudflared ? t('complete-remote-cloudflared') : ''
		]
			.filter(Boolean)
			.join(', ') || t('complete-remote-none')
	);
</script>

{#if finished}
	<div class="workspace-heading">
		<h2>{t('finished-title')}</h2>
	</div>
	<div class="step-content">
		<div class="panel success-panel" role="status">
			<div class="status-icon">✓</div>
			<div>
				<h3>{t('finished-dashboard-starting')}</h3>
				<p>{t('finished-handoff', { seconds: finished.handoffInSeconds })}</p>
				{#if finished.dashboardUrl}
					<p>
						<a class="dashboard-link" href={finished.dashboardUrl}>{finished.dashboardUrl}</a>
					</p>
				{/if}
			</div>
		</div>
	</div>
{:else}
	<StepLayout title={t('complete-title')} {onBack} {onContinue} continueKey="finish-setup">
		<div class="summary-grid">
			<div><span>{t('complete-hostname')}</span><strong>{hostname}</strong></div>
			<div><span>{t('complete-administrator')}</span><strong>{administrator}</strong></div>
			<div>
				<span>{t('complete-dashboard-address')}</span><strong>{address || '—'}</strong>
			</div>
			<div>
				<span>{t('complete-updates')}</span><strong
					>{updatePolicy === 'notify' ? t('updates-notify') : t('updates-automatic')}</strong
				>
			</div>
			<div><span>{t('complete-remote-access')}</span><strong>{remote}</strong></div>
			<div>
				<span>{t('complete-passkeys')}</span><strong
					>{access.mode === 'domain'
						? t('complete-passkeys-yes')
						: t('complete-passkeys-no')}</strong
				>
			</div>
		</div>
		<div class="complete-actions">
			<button type="button" class="secondary-button" onclick={onReboot}>{t('reboot')}</button>
			<button type="button" class="secondary-button" onclick={onShutdown}>{t('shutdown')}</button>
		</div>
	</StepLayout>
{/if}

<style>
	.complete-actions {
		display: flex;
		gap: 0.75rem;
		margin-top: 1rem;
	}
	.complete-actions button {
		flex: 1;
	}
	.success-panel h3 {
		margin: 0;
	}
	.dashboard-link {
		color: var(--foreground);
		font-weight: 700;
		overflow-wrap: anywhere;
	}
</style>
