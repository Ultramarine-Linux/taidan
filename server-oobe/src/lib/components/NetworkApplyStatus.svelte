<script lang="ts">
	import { t } from '$lib/i18n.svelte';
	import type { NetworkApplyState } from '$lib/oobe-state';

	let { apply }: { apply: NetworkApplyState } = $props();

	const titles: Record<NetworkApplyState['status'], string> = {
		applying: 'network-apply-applying',
		waiting: 'network-apply-waiting',
		kept: 'network-apply-kept',
		'rolled-back': 'network-apply-rolled-back',
		failed: 'network-apply-failed'
	};
	let pending = $derived(apply.status === 'applying' || apply.status === 'waiting');
</script>

<div
	class="panel network-apply"
	class:success-panel={apply.status === 'kept'}
	class:error-panel={apply.status === 'rolled-back' || apply.status === 'failed'}
	role={pending ? 'status' : 'alert'}
	aria-live="polite"
>
	<div class="status-icon">{pending ? '…' : apply.status === 'kept' ? '✓' : '!'}</div>
	<div>
		<h3>{t(titles[apply.status])}</h3>
		{#if apply.message && apply.status === 'failed'}<p>{apply.message}</p>{/if}
		{#if pending}
			{#if apply.urls.length > 0}
				<p>{t('network-apply-urls')}</p>
				<ul class="network-apply-urls">
					{#each apply.urls as url (url.href)}
						<li><a href={url.href}>{url.label}</a></li>
					{/each}
				</ul>
				{#if apply.automatic}<p>{t('network-apply-automatic')}</p>{/if}
			{:else}
				<p>{t('network-apply-no-urls')}</p>
			{/if}
			<p>{t('network-apply-rollback', { seconds: apply.rollbackInSeconds })}</p>
		{/if}
	</div>
</div>

<style>
	.network-apply h3 {
		margin: 0;
	}

	.network-apply p {
		margin: 0.4rem 0 0;
	}

	.network-apply-urls {
		margin: 0.4rem 0 0;
		padding-left: 1.2rem;
		font-size: 0.85rem;
		overflow-wrap: anywhere;
	}
</style>
