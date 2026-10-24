<script lang="ts">
	import { getContext } from 'svelte';
	import { t } from '$lib/i18n.svelte';
	import { stepContextKey, type StepContext } from './Shell.svelte';
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

	const step = getContext<StepContext | undefined>(stepContextKey);
</script>

<div class="actions-bar">
	<span class="step-counter">
		{#if step}{t('step-counter', { current: step.index + 1, total: step.total })}{/if}
	</span>
	<div class="actions">
		{#if showBack}
			<button class="secondary-button" type="button" onclick={onBack}>{t('prev')}</button>
		{/if}
		<button class="primary-button" type="button" onclick={onContinue} disabled={!canContinue}
			>{t(continueKey)}</button
		>
	</div>
</div>
