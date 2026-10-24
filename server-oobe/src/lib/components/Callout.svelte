<script lang="ts">
	import type { Snippet } from 'svelte';
	import Icon from './Icon.svelte';

	export type CalloutTone = 'neutral' | 'info' | 'success' | 'warning' | 'danger' | 'pending';

	let {
		tone = 'neutral',
		title,
		class: className = '',
		role,
		children
	}: {
		tone?: CalloutTone;
		title: string;
		class?: string;
		role?: 'status' | 'alert' | 'note';
		children?: Snippet;
	} = $props();

	const tones: Record<CalloutTone, { panel: string; icon: string }> = {
		neutral: { panel: '', icon: 'info' },
		pending: { panel: 'info-panel', icon: 'clock' },
		info: { panel: 'info-panel', icon: 'info' },
		success: { panel: 'success-panel', icon: 'check-circle' },
		warning: { panel: 'warning-panel', icon: 'alert-triangle' },
		danger: { panel: 'error-panel', icon: 'alert-circle' }
	};
</script>

<div class="panel {tones[tone].panel} {className}" {role}>
	<Icon name={tones[tone].icon} class="status-icon" />
	<div>
		<h3>{title}</h3>
		{@render children?.()}
	</div>
</div>
