<script lang="ts">
	import StepLayout from '$lib/components/StepLayout.svelte';
	import { t } from '$lib/i18n.svelte';
	let deviceName = $state('');
	let touched = $state(false);
	let { hostname = $bindable(''), onBack, onContinue } = $props();
	let hostnameError = $derived(
		touched &&
			!/^(([a-z0-9]|[a-z0-9][a-z0-9-]*[a-z0-9])\.)*[a-z0-9](?:[a-z0-9-]*[a-z0-9])?$/.test(hostname)
			? t('hostname-error')
			: ''
	);
	function derive(value: string) {
		if (touched) return;

		hostname = value
			.trim()
			.toLowerCase()
			.replace(/[^a-z0-9]+/g, '-')
			.replace(/^-+|-+$/g, '');
	}
</script>

<StepLayout
	title={t('device-name-title')}
	{onBack}
	{onContinue}
	canContinue={Boolean(hostname) && !hostnameError}
>
	<label class="field-label" for="device-name"
		>{t('device-name-label')}<input
			class="text-input"
			id="device-name"
			bind:value={deviceName}
			oninput={() => derive(deviceName)}
		/></label
	>
	<label class="field-label" for="hostname"
		>{t('hostname-label')}<input
			class="text-input"
			id="hostname"
			bind:value={hostname}
			oninput={() => (touched = true)}
			aria-describedby="hostname-error"
		/></label
	>
	{#if hostnameError}<p class="field-error" id="hostname-error">{hostnameError}</p>{/if}
</StepLayout>
