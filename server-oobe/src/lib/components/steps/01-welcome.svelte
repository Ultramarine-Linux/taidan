<script lang="ts">
	import StepLayout from '$lib/components/StepLayout.svelte';
	import { t } from '$lib/i18n.svelte';
	import { FIXTURE_SETUP_CODE } from '$lib/fixture';
	let {
		onBack,
		onContinue,
		ipAddresses = [],
		needsSetupCode = false,
		setupCode = $bindable(''),
		setupCodeError = ''
	}: {
		onBack: () => void;
		onContinue: () => void;
		ipAddresses?: string[];
		needsSetupCode?: boolean;
		setupCode?: string;
		setupCodeError?: string;
	} = $props();
</script>

<StepLayout
	title={t('welcome-title')}
	{onBack}
	{onContinue}
	showBack={false}
	canContinue={!needsSetupCode || setupCode.trim().length > 0}
>
	<div class="panel success-panel">
		<div class="status-icon">✓</div>
		<div>
			<h3>{t('welcome-ready-title')}</h3>
			<p>{t('welcome-ready-description')}</p>
			{#if ipAddresses.length > 0}
				<p class="ip-addresses">
					<strong>{t('welcome-ip-addresses')}</strong>
					{ipAddresses.join(', ')}
				</p>
			{/if}
		</div>
	</div>

	{#if needsSetupCode}
		<form
			class="setup-code"
			onsubmit={(event) => {
				event.preventDefault();
				if (setupCode.trim()) onContinue();
			}}
		>
			<label class="field-label" for="setup-code">
				{t('setup-code-label')}
				<input
					class="text-input"
					id="setup-code"
					bind:value={setupCode}
					placeholder="XXXX-XXXX"
					autocomplete="one-time-code"
					autocapitalize="characters"
					spellcheck="false"
					aria-describedby="setup-code-description{setupCodeError ? ' setup-code-error' : ''}"
					aria-invalid={setupCodeError ? 'true' : undefined}
				/>
				<small id="setup-code-description">
					{t('setup-code-description')}
					{#if import.meta.env.VITE_FIXTURE_API === 'true'}
						{t('setup-code-fixture', { code: FIXTURE_SETUP_CODE })}
					{/if}
				</small>
			</label>
			{#if setupCodeError}
				<p class="field-error" id="setup-code-error" role="alert">{setupCodeError}</p>
			{/if}
		</form>
	{/if}
</StepLayout>

<style>
	.setup-code {
		margin-top: 1rem;
	}

	.setup-code small {
		color: var(--muted-foreground);
		font-size: 0.75rem;
		font-weight: 400;
		line-height: 1.4;
	}
</style>
