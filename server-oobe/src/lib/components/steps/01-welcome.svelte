<script lang="ts">
	import Callout from '$lib/components/Callout.svelte';
	import StepLayout from '$lib/components/StepLayout.svelte';
	import { t } from '$lib/i18n.svelte';
	import { FIXTURE_SETUP_CODE } from '$lib/fixture';
	let {
		onBack,
		onContinue,
		urls = [],
		needsSetupCode = false,
		setupCode = $bindable(''),
		setupCodeError = ''
	}: {
		onBack: () => void;
		onContinue: () => void;
		/** Where other devices reach this OOBE, e.g. `https://[2001:db8::1]:6623`. */
		urls?: string[];
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
	<Callout tone="success" title={t('welcome-ready-title')}>
		<p>{t('welcome-ready-description')}</p>
	</Callout>

	{#if urls.length > 0}
		<div class="ip-addresses">
			<p class="ip-addresses-label">{t('welcome-ip-addresses')}</p>
			<ul class="summary-list">
				{#each urls as url (url)}
					<li><span class="code-value">{url}</span></li>
				{/each}
			</ul>
		</div>
	{/if}

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
					class="mono-input"
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
	.ip-addresses {
		display: grid;
		gap: 0.5rem;
	}

	.ip-addresses-label {
		color: var(--foreground);
		font-size: 0.875rem;
		font-weight: 500;
	}

	.ip-addresses ul {
		padding: 0;
		list-style: none;
	}

	.ip-addresses li {
		padding: 0.625rem 1rem;
		overflow-wrap: anywhere;
	}

	.ip-addresses li + li {
		border-top: 1px solid var(--border);
	}

	.setup-code {
		display: grid;
		gap: 0.5rem;
	}

	.setup-code input {
		max-width: 16rem;
		height: 2.5rem;
		font-size: 1rem;
	}
</style>
