<script lang="ts">
	import StepLayout from '$lib/components/StepLayout.svelte';
	import { t } from '$lib/i18n.svelte';
	let {
		onBack,
		onContinue,
		onReboot,
		onShutdown,
		hostname = 'ultramarine-server',
		administrator = 'Not configured',
		dashboardDomain = '',
		dashboardPort = 3972
	}: {
		onBack: () => void;
		onContinue: () => void;
		onReboot: () => void;
		onShutdown: () => void;
		hostname?: string;
		administrator?: string;
		dashboardDomain?: string;
		dashboardPort?: number;
	} = $props();
</script>

<StepLayout title={t('complete-title')} {onBack} {onContinue} continueKey="go-to-dashboard">
	<div class="summary-grid">
		<div><span>{t('complete-hostname')}</span><strong>{hostname}</strong></div>
		<div><span>{t('complete-administrator')}</span><strong>{administrator}</strong></div>
		<div>
			<span>{t('complete-dashboard-domain')}</span><strong
				>{dashboardDomain || t('dashboard-domain-local')}</strong
			>
		</div>
		<div><span>{t('complete-dashboard-port')}</span><strong>{dashboardPort}</strong></div>
	</div>
	<div class="complete-actions">
		<button type="button" class="primary-button" onclick={onReboot}>{t('reboot')}</button>
		<button type="button" class="secondary-button" onclick={onShutdown}>{t('shutdown')}</button>
	</div>
</StepLayout>

<style>
	.complete-actions {
		display: flex;
		gap: 0.75rem;
		margin-top: 1rem;
	}
	.complete-actions button {
		flex: 1;
	}
</style>
