<script lang="ts">
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

<nav class="steps-card" aria-label="Setup steps">
	<div class="card-heading">
		<h2>{t('get-server-ready')}</h2>
	</div>
	<div class="step-list">
		{#each steps as step, index}
			<button
				class:active={selectedStep === step.id}
				class:unavailable={!isClickable(step)}
				class="step"
				data-step-id={step.id}
				aria-current={selectedStep === step.id ? 'step' : undefined}
				type="button"
				onclick={() => {
					if (isClickable(step)) onSelect(step.id);
				}}
			>
				<span class:complete={step.status === 'complete'} class="step-number">
					{step.status === 'complete' ? '✓' : index + 1}
				</span>
				<span class="step-copy">
					<strong>{t(step.label)}</strong>
					<small>{t(step.description)}</small>
				</span>
			</button>
		{/each}
	</div>
</nav>

<style>
	.steps-card {
		border: 1px solid var(--border);
		background: var(--card);
		height: calc(100vh - 10rem);
		overflow-y: auto;
		padding: 1.5rem;
	}

	.step-list {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}

	.step {
		display: flex;
		align-items: center;
		gap: 0.6rem;
		border: 1px solid transparent;
		background: transparent;
		color: var(--foreground);
		padding: 0.5rem 0.6rem;
		text-align: left;
		cursor: pointer;
		transition:
			background-color 120ms ease,
			border-color 120ms ease;
	}

	.step:hover {
		background: var(--muted);
	}

	.step.active {
		border-color: var(--primary);
		background: color-mix(in srgb, var(--primary) 8%, transparent);
	}

	.step.unavailable {
		cursor: default;
	}

	.step.unavailable:hover {
		background: transparent;
	}

	.step-number {
		display: grid;
		place-items: center;
		width: 1.5rem;
		height: 1.5rem;
		flex: 0 0 auto;
		border: 1px solid var(--border);
		font-size: 0.7rem;
		font-weight: 600;
		line-height: 1;
	}

	.step-number.complete {
		background: var(--primary);
		color: var(--primary-foreground);
		border-color: var(--primary);
	}

	.step-copy {
		display: flex;
		flex-direction: column;
		gap: 0.1rem;
	}

	.step-copy strong {
		font-size: 0.8rem;
		font-weight: 600;
	}

	.step-copy small {
		font-size: 0.7rem;
		color: var(--muted-foreground);
		line-height: 1.3;
	}

	@media (max-width: 700px) {
		.steps-card {
			display: none;
		}
	}
</style>
