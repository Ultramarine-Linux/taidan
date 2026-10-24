<script lang="ts">
	import Icon from './Icon.svelte';
	import { t } from '$lib/i18n.svelte';
	import type { OobeStep, StepId } from '$lib/oobe-state';

	let {
		steps,
		selectedStep,
		onSelect
	}: { steps: OobeStep[]; selectedStep: StepId; onSelect: (id: StepId) => void } = $props();

	function isClickable(step: OobeStep): boolean {
		return step.status === 'complete' || selectedStep === step.id;
	}
</script>

<nav class="step-nav" aria-label={t('setup-steps')}>
	<p class="nav-group-label">{t('get-server-ready')}</p>
	<ol class="step-list">
		{#each steps as step, index (step.id)}
			{@const active = selectedStep === step.id}
			<li>
				<button
					class="step"
					class:active
					data-step-id={step.id}
					aria-current={active ? 'step' : undefined}
					aria-disabled={isClickable(step) ? undefined : 'true'}
					type="button"
					onclick={() => {
						if (isClickable(step)) onSelect(step.id);
					}}
				>
					<span
						class="step-number"
						class:complete={step.status === 'complete'}
						class:failed={step.status === 'failed'}
					>
						{#if step.status === 'complete'}
							<Icon name="check" />
							<span class="sr-only">{t('step-complete-sr')}</span>
						{:else}
							{index + 1}
						{/if}
					</span>
					<span class="step-label">{t(step.label)}</span>
				</button>
			</li>
		{/each}
	</ol>
</nav>
