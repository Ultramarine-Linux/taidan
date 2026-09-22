<script lang="ts">
	import { t } from '$lib/i18n.svelte';
	let {
		canContinue = true,
		onBack,
		onContinue,
		continueKey = 'next',
		showBack = true
	}: {
		canContinue?: boolean;
		onBack: () => void;
		onContinue: () => void;
		continueKey?: string;
		showBack?: boolean;
	} = $props();
</script>

<div class="actions">
	{#if showBack}
		<button class="secondary-button" type="button" onclick={onBack}>{t('prev')}</button>
	{/if}
	<button class="primary-button" type="button" onclick={onContinue} disabled={!canContinue}
		>{t(continueKey)}</button
	>
</div>
