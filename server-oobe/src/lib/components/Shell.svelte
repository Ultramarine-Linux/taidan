<script lang="ts">
	import Header from './Header.svelte';
	import StepNavigation from './StepNavigation.svelte';
	import type { OobeStep, StepId } from '$lib/oobe-state';

	let {
		children,
		steps,
		selectedStep,
		onSelectStep
	}: {
		children: import('svelte').Snippet;
		steps: OobeStep[];
		selectedStep: StepId;
		onSelectStep: (id: StepId) => void;
	} = $props();

	let theme = $state<'light' | 'dark'>('light');

	$effect(() => {
		const storedTheme = localStorage.getItem('oobe-theme');
		theme = storedTheme === 'dark' ? 'dark' : 'light';
	});

	$effect(() => {
		document.documentElement.classList.toggle('dark', theme === 'dark');
	});

	function toggleTheme() {
		theme = theme === 'light' ? 'dark' : 'light';
		localStorage.setItem('oobe-theme', theme);
	}
</script>

<main class="shell">
	<Header {theme} onToggleTheme={toggleTheme} />
	<div class="content-grid">
		<StepNavigation {steps} {selectedStep} onSelect={onSelectStep} />
		<section class="workspace" aria-live="polite">
			{@render children()}
		</section>
	</div>
</main>
