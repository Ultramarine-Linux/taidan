<script lang="ts">
	import { t } from '$lib/i18n.svelte';
	import StepActions from './StepActions.svelte';
	let {
		title,
		children,
		onBack,
		onContinue,
		canContinue = true,
		continueKey = 'next',
		showBack = true
	}: {
		title: string;
		children: import('svelte').Snippet;
		onBack: () => void;
		onContinue: () => void;
		canContinue?: boolean;
		continueKey?: string;
		showBack?: boolean;
	} = $props();
</script>

<div class="workspace-heading">
	<h2>{title}</h2>
	{#if import.meta.env.VITE_FIXTURE_API === 'true'}
		<span class="status-pill">{t('fixture-mode')}</span>
	{/if}
</div>
<div class="step-content">{@render children()}</div>
<StepActions {onBack} {onContinue} {canContinue} {continueKey} {showBack} />
