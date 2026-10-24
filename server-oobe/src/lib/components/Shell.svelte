<script lang="ts" module>
	import type { StepId } from '$lib/oobe-state';

	/** The step being shown, for StepLayout's heading and step counter. */
	export type StepContext = {
		readonly id: StepId;
		readonly index: number;
		readonly total: number;
		readonly description: string;
	};

	export const stepContextKey = Symbol('oobe-step');
</script>

<script lang="ts">
	import Header from './Header.svelte';
	import StepNavigation from './StepNavigation.svelte';
	import Icon from './Icon.svelte';
	import { appearance } from '$lib/appearance.svelte';
	import { t } from '$lib/i18n.svelte';
	import type { OobeStep } from '$lib/oobe-state';
	import { onMount, setContext, type Snippet } from 'svelte';

	let {
		children,
		banners,
		steps,
		selectedStep,
		onSelectStep
	}: {
		children: Snippet;
		/** Notices shown above the step, like the Dashboard's shell banners. */
		banners?: Snippet;
		steps: OobeStep[];
		selectedStep: StepId;
		onSelectStep: (id: StepId) => void;
	} = $props();

	let drawer = $state<HTMLDialogElement>();

	let index = $derived(
		Math.max(
			0,
			steps.findIndex((step) => step.id === selectedStep)
		)
	);
	let current = $derived(steps[index]);
	let completed = $derived(steps.filter((step) => step.status === 'complete').length);
	let progress = $derived(steps.length ? Math.round((completed / steps.length) * 100) : 0);

	setContext<StepContext>(stepContextKey, {
		get id() {
			return selectedStep;
		},
		get index() {
			return index;
		},
		get total() {
			return steps.length;
		},
		get description() {
			return current ? t(current.description) : '';
		}
	});

	onMount(() => appearance.load());

	$effect(() => {
		appearance.apply();
	});

	function select(id: StepId) {
		drawer?.close();
		onSelectStep(id);
	}
</script>

{#snippet progressCard()}
	<div class="progress-card">
		<p class="progress-card-title">{t('setup-progress-title')}</p>
		<p class="progress-card-detail">
			{t('setup-progress', { completed, total: steps.length })}
		</p>
		<div class="progress-track" aria-hidden="true">
			<div class="progress-fill" style:width="{progress}%"></div>
		</div>
	</div>
{/snippet}

<div class="app-shell">
	<aside class="sidebar" aria-label={t('setup-sidebar')}>
		<div class="sidebar-brand">
			<img class="brand-logo" src="/ultramarine-logo.svg" alt="" />
			<span class="brand-name">{t('brand-name')}</span>
		</div>
		{@render progressCard()}
		<div class="sidebar-scroll">
			<StepNavigation {steps} {selectedStep} onSelect={select} />
		</div>
		<div class="sidebar-footer">{t('sidebar-footer')}</div>
	</aside>

	<div class="app-main">
		<Header
			stepLabel={current ? t(current.label) : ''}
			onOpenNavigation={() => drawer?.showModal()}
		/>
		<div class="mobile-progress" aria-hidden="true">
			<div class="progress-fill" style:width="{progress}%"></div>
		</div>
		<main class="workspace-main">
			{@render banners?.()}
			<section class="workspace" aria-live="polite">
				{@render children()}
			</section>
		</main>
	</div>
</div>

<dialog
	class="nav-drawer"
	bind:this={drawer}
	aria-label={t('setup-steps')}
	onclick={(event) => {
		// A click on the backdrop lands on the dialog itself.
		if (event.target === drawer) drawer.close();
	}}
>
	<div class="nav-drawer-header">
		<div>
			<img class="brand-logo" src="/ultramarine-logo.svg" alt="" />
			<span class="brand-name">{t('brand-name')}</span>
		</div>
		<button
			class="icon-button borderless"
			type="button"
			onclick={() => drawer?.close()}
			aria-label={t('close-steps')}
		>
			<Icon name="x" />
		</button>
	</div>
	{@render progressCard()}
	<div class="sidebar-scroll">
		<StepNavigation {steps} {selectedStep} onSelect={select} />
	</div>
</dialog>
