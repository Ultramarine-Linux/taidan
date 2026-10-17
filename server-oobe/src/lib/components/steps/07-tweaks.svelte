<script lang="ts">
	import StepLayout from '$lib/components/StepLayout.svelte';
	import { t } from '$lib/i18n.svelte';
	let {
		settings = $bindable({ ssh: true, firewall: true, automaticUpdates: true }),
		onBack,
		onContinue
	}: {
		settings?: { ssh: boolean; firewall: boolean; automaticUpdates: boolean };
		onBack: () => void;
		onContinue: () => void;
	} = $props();
</script>

<StepLayout title={t('defaults-title')} {onBack} {onContinue}>
	<p class="lead">{t('defaults-description')}</p>
	<div class="option-list">
		<label class="toggle-option"
			><span
				><strong>{t('defaults-ssh')}</strong><small>{t('defaults-ssh-description')}</small></span
			><input type="checkbox" bind:checked={settings.ssh} /></label
		>
		<label class="toggle-option"
			><span
				><strong>{t('defaults-firewall')}</strong><small>{t('defaults-firewall-description')}</small
				></span
			><input type="checkbox" bind:checked={settings.firewall} /></label
		>
		<label class="toggle-option"
			><span
				><strong>{t('automatic-updates')}</strong><small>{t('automatic-updates-description')}</small
				></span
			><input type="checkbox" bind:checked={settings.automaticUpdates} /></label
		>
	</div>
</StepLayout>
