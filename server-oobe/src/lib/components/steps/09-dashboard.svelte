<script lang="ts">
	import StepLayout from '$lib/components/StepLayout.svelte';
	import { t } from '$lib/i18n.svelte';

	let {
		dashboardDomain = $bindable(''),
		dashboardPort = $bindable(3972),
		onBack,
		onContinue
	}: {
		dashboardDomain?: string;
		dashboardPort?: number;
		onBack: () => void;
		onContinue: () => void;
	} = $props();

	let dashboardAddress = $derived(
		`${dashboardDomain?.trim() || 'localhost'}:${dashboardPort || 3972}`
	);
</script>

<StepLayout
	title={t('hosting-title')}
	{onBack}
	{onContinue}
	canContinue={dashboardPort > 0 && dashboardPort < 65536}
	continueKey="review-setup"
>
	<p class="lead">{t('hosting-local-description')}</p>

	<label class="field-label" for="dashboard-domain">
		{t('dashboard-domain')}
		<input
			class="text-input"
			id="dashboard-domain"
			bind:value={dashboardDomain}
			placeholder="dashboard.example.com"
			autocomplete="url"
		/>
		<small>{t('dashboard-domain-description')}</small>
	</label>

	<label class="field-label" for="dashboard-port">
		<span class="port-label">
			{t('dashboard-port')}
			<span class="port-info">{t('dashboard-port-info')}</span>
		</span>
		<input
			class="text-input"
			id="dashboard-port"
			type="number"
			min="1"
			max="65535"
			bind:value={dashboardPort}
		/>
		<small>{t('dashboard-port-description')}</small>
	</label>

	<div class="panel dashboard-preview" aria-live="polite">
		<div>
			<h3>{t('dashboard-address-preview')}</h3>
			<p>{t('dashboard-address-description')}</p>
		</div>
		<strong>http://{dashboardAddress}</strong>
	</div>
</StepLayout>

<style>
	.lead {
		margin-bottom: 1rem;
		color: var(--muted-foreground);
		line-height: 1.5;
	}

	.field-label small {
		color: var(--muted-foreground);
		font-size: 0.75rem;
		font-weight: 400;
		line-height: 1.4;
	}

	.port-label {
		display: inline-flex;
		align-items: center;
		flex-wrap: wrap;
		gap: 0.4rem;
	}

	.port-info {
		color: var(--muted-foreground);
		font-size: 0.72rem;
		font-weight: 400;
		margin-top: 0.1rem;
		margin-left: 0.6rem;
	}

	.dashboard-preview {
		align-items: center;
		justify-content: space-between;
		margin-top: 1rem;
	}

	.dashboard-preview h3,
	.dashboard-preview p {
		margin: 0;
	}

	.dashboard-preview p {
		margin-top: 0.35rem;
	}

	.dashboard-preview strong {
		color: var(--foreground);
		font-size: 0.85rem;
		white-space: nowrap;
	}

	@media (max-width: 620px) {
		.dashboard-preview {
			align-items: flex-start;
			flex-direction: column;
			gap: 0.5rem;
		}
	}
</style>
