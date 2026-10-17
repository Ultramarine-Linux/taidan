<script lang="ts">
	import StepLayout from '$lib/components/StepLayout.svelte';
	import { t } from '$lib/i18n.svelte';
	let {
		onBack,
		onContinue,
		ipAddresses = []
	}: { onBack: () => void; onContinue: () => void; ipAddresses?: string[] } = $props();
</script>

<StepLayout title={t('welcome-title')} {onBack} {onContinue} showBack={false}>
	<div class="panel success-panel">
		<div class="status-icon">✓</div>
		<div>
			<h3>{t('welcome-ready-title')}</h3>
			<p>{t('welcome-ready-description')}</p>
			{#if ipAddresses.length > 0}
				<p class="ip-addresses">
					<strong>{t('welcome-ip-addresses')}</strong>
					{ipAddresses.join(', ')}
				</p>
			{/if}
		</div>
	</div>
</StepLayout>
