<script lang="ts">
	import { getContext, type Snippet } from 'svelte';
	import StepActions from './StepActions.svelte';
	import { stepContextKey, type StepContext } from './Shell.svelte';
	let {
		title,
		description,
		children,
		onBack,
		onContinue,
		canContinue = true,
		continueKey = 'next',
		showBack = true,
		showActions = true
	}: {
		title: string;
		/** Shown under the title. Defaults to the step's description. */
		description?: string;
		children: Snippet;
		onBack?: () => void;
		onContinue?: () => void;
		canContinue?: boolean;
		continueKey?: string;
		showBack?: boolean;
		showActions?: boolean;
	} = $props();

	const step = getContext<StepContext | undefined>(stepContextKey);

	// Inputs where Enter means something else, such as typing a newline.
	const enterIgnored = new Set(['button', 'submit', 'reset', 'file', 'image', 'color', 'range']);

	/** Enter in a field continues, like submitting a form. */
	function onKeydown(event: KeyboardEvent) {
		if (event.key !== 'Enter' || event.defaultPrevented || event.isComposing) return;
		if (event.shiftKey || event.ctrlKey || event.altKey || event.metaKey) return;
		const target = event.target;
		if (!(target instanceof HTMLInputElement) || enterIgnored.has(target.type)) return;
		// Only fields of this step; parts of a step can opt out (Wi-Fi connects instead).
		if (!target.closest('.step-scroll') || target.closest('[data-enter-continue="off"]')) return;
		event.preventDefault();
		if (showActions && canContinue) onContinue?.();
	}

	let subtitle = $derived.by(() => {
		const text = description ?? step?.description ?? '';
		return text === title ? '' : text;
	});
</script>

<svelte:window onkeydown={onKeydown} />

<div class="step-scroll">
	<div class="step-body">
		<div class="workspace-heading">
			<h2>{title}</h2>
			{#if subtitle}<p>{subtitle}</p>{/if}
		</div>
		<div class="step-content">{@render children()}</div>
	</div>
</div>
{#if showActions && onBack && onContinue}
	<StepActions {onBack} {onContinue} {canContinue} {continueKey} {showBack} />
{/if}
