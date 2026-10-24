<script lang="ts">
	import Callout from '$lib/components/Callout.svelte';
	import StepLayout from '$lib/components/StepLayout.svelte';
	import { t } from '$lib/i18n.svelte';
	import { DNS_PROVIDERS, dnsProvider, validateDnsCredentials } from '$lib/dns-providers';
	import { ipFamily, stripBrackets, urlHost } from '$lib/net';
	import type { AccessForm } from '$lib/oobe-state';
	let {
		form = $bindable<AccessForm>(),
		ipAddresses = [],
		onBack,
		onContinue
	}: {
		form: AccessForm;
		ipAddresses?: string[];
		onBack: () => void;
		onContinue: () => void;
	} = $props();

	const domainPattern = /^(?=.{1,253}$)([a-z0-9]([a-z0-9-]{0,61}[a-z0-9])?\.)+[a-z]{2,63}$/;

	let provider = $derived(dnsProvider(form.dnsProvider));
	// IPv4 or IPv6 (the latter with or without brackets).
	let hostname = $derived(stripBrackets(form.hostname.trim().toLowerCase().replace(/\.$/, '')));
	let hostnameValid = $derived(
		form.mode === 'domain' ? domainPattern.test(hostname) : ipFamily(hostname) !== 0
	);
	let canContinue = $derived(
		hostnameValid &&
			(form.mode === 'ip' ||
				form.challenge === 'http-01' ||
				!validateDnsCredentials(form.dnsProvider, form.dnsCredentials))
	);
	let address = $derived(hostname ? `https://${urlHost(hostname)}` : '');

	function setMode(mode: AccessForm['mode']) {
		form.mode = mode;
		form.hostname = mode === 'ip' ? (ipAddresses[0] ?? '') : '';
	}

	function setProvider(id: string) {
		// Credentials belong to one provider; never carry them over.
		form.dnsProvider = id;
		form.dnsCredentials = {};
	}
</script>

<StepLayout
	title={t('access-title')}
	description={t('access-description')}
	{onBack}
	{onContinue}
	{canContinue}
	continueKey="review-setup"
>
	<section class="form-section">
		<fieldset>
			<legend>{t('access-mode')}</legend>
			<label class="toggle-option" for="access-domain">
				<span>
					<strong>{t('access-domain')}</strong>
					<small>{t('access-domain-description')}</small>
				</span>
				<input
					id="access-domain"
					type="radio"
					name="access-mode"
					checked={form.mode === 'domain'}
					onchange={() => setMode('domain')}
				/>
			</label>
			<label class="toggle-option" for="access-ip">
				<span>
					<strong>{t('access-ip')}</strong>
					<small>{t('access-ip-description')}</small>
				</span>
				<input
					id="access-ip"
					type="radio"
					name="access-mode"
					checked={form.mode === 'ip'}
					onchange={() => setMode('ip')}
				/>
			</label>
		</fieldset>

		{#if form.mode === 'domain'}
			<label class="field-label" for="access-hostname">
				{t('access-hostname')}
				<input
					id="access-hostname"
					bind:value={form.hostname}
					placeholder="server.example.com"
					autocomplete="off"
					spellcheck="false"
				/>
			</label>
			<fieldset>
				<legend>{t('access-challenge')}</legend>
				<label class="toggle-option" for="challenge-http">
					<span>
						<strong>{t('access-challenge-http')}</strong>
						<small>{t('access-challenge-http-description')}</small>
					</span>
					<input
						id="challenge-http"
						type="radio"
						name="access-challenge"
						value="http-01"
						bind:group={form.challenge}
					/>
				</label>
				<label class="toggle-option" for="challenge-dns">
					<span>
						<strong>{t('access-challenge-dns')}</strong>
						<small>{t('access-challenge-dns-description')}</small>
					</span>
					<input
						id="challenge-dns"
						type="radio"
						name="access-challenge"
						value="dns-01"
						bind:group={form.challenge}
					/>
				</label>
			</fieldset>
			{#if form.challenge === 'dns-01'}
				<label class="field-label" for="dns-provider">
					{t('access-dns-provider')}
					<select
						id="dns-provider"
						value={form.dnsProvider}
						onchange={(event) => setProvider(event.currentTarget.value)}
					>
						<option value="" disabled>{t('access-dns-provider-select')}</option>
						{#each DNS_PROVIDERS as option}
							<option value={option.id}>{option.label}</option>
						{/each}
					</select>
				</label>
				{#if provider}
					{#each provider.fields as field (field.name)}
						<label class="field-label" for="dns-{field.name}">
							{field.label}{field.optional ? ` (${t('optional')})` : ''}
							<input
								id="dns-{field.name}"
								type={field.secret ? 'password' : 'text'}
								autocomplete="off"
								spellcheck="false"
								bind:value={form.dnsCredentials[field.name]}
							/>
						</label>
					{/each}
					<p class="note">{t('access-dns-credentials-note')}</p>
				{/if}
			{/if}
		{:else}
			<label class="field-label" for="access-ip-address">
				{t('access-ip-address')}
				{#if ipAddresses.length > 0}
					<select id="access-ip-address" bind:value={form.hostname}>
						{#each ipAddresses as ip}
							<option value={ip}>{ip}</option>
						{/each}
					</select>
				{:else}
					<input
						id="access-ip-address"
						bind:value={form.hostname}
						placeholder="192.0.2.10"
						autocomplete="off"
						spellcheck="false"
					/>
				{/if}
			</label>
			<Callout tone="warning" role="note" title={t('access-ip-passkeys-title')}>
				<p>{t('access-ip-passkeys')}</p>
			</Callout>
		{/if}
	</section>

	{#if address}
		<div class="panel info-panel dashboard-preview" aria-live="polite">
			<div>
				<h3>{t('dashboard-address-preview')}</h3>
				<p>{t('dashboard-address-description')}</p>
			</div>
			<strong class="code-value">{address}</strong>
		</div>
	{/if}
</StepLayout>

<style>
	.dashboard-preview {
		align-items: center;
		justify-content: space-between;
	}

	.dashboard-preview strong {
		color: var(--foreground);
		font-weight: 500;
		overflow-wrap: anywhere;
	}

	@media (max-width: 620px) {
		.dashboard-preview {
			align-items: flex-start;
			flex-direction: column;
			gap: 0.5rem;
		}
	}
</style>
