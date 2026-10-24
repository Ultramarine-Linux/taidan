<script lang="ts">
	import Callout from './Callout.svelte';
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

<Callout
	class="network-apply"
	tone={pending ? 'pending' : apply.status === 'kept' ? 'success' : 'danger'}
	role={pending ? 'status' : 'alert'}
	title={t(titles[apply.status])}
>
	{#if apply.message && apply.status === 'failed'}<p>{apply.message}</p>{/if}
	{#if pending}
		{#if apply.urls.length > 0}
			<p>{t('network-apply-urls')}</p>
			<ul class="network-apply-urls">
				{#each apply.urls as url (url.href)}
					<li><a class="code-value" href={url.href}>{url.label}</a></li>
				{/each}
			</ul>
			{#if apply.automatic}<p>{t('network-apply-automatic')}</p>{/if}
		{:else}
			<p>{t('network-apply-no-urls')}</p>
		{/if}
		<p>{t('network-apply-rollback', { seconds: apply.rollbackInSeconds })}</p>
	{/if}
</Callout>

<style>
	.network-apply-urls {
		overflow-wrap: anywhere;
	}

	.network-apply-urls a {
		color: var(--foreground);
		font-weight: 500;
	}
</style>
