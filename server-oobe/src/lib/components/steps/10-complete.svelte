<script lang="ts">
	import Callout from '$lib/components/Callout.svelte';
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
	<StepLayout title={t('finished-title')} description="" showActions={false}>
		<Callout tone="success" role="status" title={t('finished-dashboard-starting')}>
			<p>{t('finished-handoff', { seconds: finished.handoffInSeconds })}</p>
			{#if finished.dashboardUrl}
				<p>
					<a class="dashboard-link code-value" href={finished.dashboardUrl}
						>{finished.dashboardUrl}</a
					>
				</p>
			{/if}
		</Callout>
	</StepLayout>
{:else}
	<StepLayout title={t('complete-title')} {onBack} {onContinue} continueKey="finish-setup">
		<dl class="summary-list">
			<div>
				<dt>{t('complete-hostname')}</dt>
				<dd>{hostname}</dd>
			</div>
			<div>
				<dt>{t('complete-administrator')}</dt>
				<dd>{administrator}</dd>
			</div>
			<div>
				<dt>{t('complete-dashboard-address')}</dt>
				<dd class="code-value">{address || '—'}</dd>
			</div>
			<div>
				<dt>{t('complete-updates')}</dt>
				<dd>{updatePolicy === 'notify' ? t('updates-notify') : t('updates-automatic')}</dd>
			</div>
			<div>
				<dt>{t('complete-remote-access')}</dt>
				<dd>{remote}</dd>
			</div>
			<div>
				<dt>{t('complete-passkeys')}</dt>
				<dd>
					{access.mode === 'domain' ? t('complete-passkeys-yes') : t('complete-passkeys-no')}
				</dd>
			</div>
		</dl>
		<section class="power-section" aria-labelledby="power-heading">
			<div>
				<h3 id="power-heading">{t('complete-power-title')}</h3>
				<p>{t('complete-power-description')}</p>
			</div>
			<div class="inline-actions">
				<button type="button" class="secondary-button button-sm" onclick={onReboot}
					>{t('reboot')}</button
				>
				<button type="button" class="secondary-button button-sm" onclick={onShutdown}
					>{t('shutdown')}</button
				>
			</div>
		</section>
	</StepLayout>
{/if}

<style>
	.power-section {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		justify-content: space-between;
		gap: 0.75rem 1rem;
		border-top: 1px solid var(--border);
		padding-top: 1.25rem;
	}

	.power-section h3 {
		font-size: 0.875rem;
		font-weight: 600;
	}

	.power-section p {
		margin-top: 0.125rem;
		color: var(--muted-foreground);
		font-size: 0.75rem;
	}

	.dashboard-link {
		color: var(--foreground);
		font-weight: 600;
		text-decoration: underline;
		overflow-wrap: anywhere;
	}
</style>
