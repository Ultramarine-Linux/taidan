<script lang="ts">
	import { t } from '$lib/i18n.svelte';
	import { isTunneled, type Eap8021xForm, type EapMethod, type Phase2Method } from '$lib/eap-8021x';

	type PemField = 'caCert' | 'clientCert' | 'privateKey';

	let {
		idPrefix,
		settings = $bindable(),
		eapMethods = ['peap', 'ttls', 'tls'],
		phase2Methods = ['mschapv2', 'pap']
	}: {
		/** Prefix for element ids, e.g. `wifi` or `wired`. */
		idPrefix: string;
		settings: Eap8021xForm;
		eapMethods?: EapMethod[];
		phase2Methods?: Phase2Method[];
	} = $props();

	let pemErrors = $state<Partial<Record<PemField, string>>>({});

	const pemUploads: { field: PemField; label: string; accept: string }[] = $derived([
		{
			field: 'caCert',
			label: t('network-8021x-ca-cert'),
			accept: '.pem,.crt,.cer,application/x-pem-file'
		},
		...(settings.eap === 'tls'
			? [
					{
						field: 'clientCert' as const,
						label: t('network-8021x-client-cert'),
						accept: '.pem,.crt,.cer,application/x-pem-file'
					},
					{
						field: 'privateKey' as const,
						label: t('network-8021x-private-key'),
						accept: '.pem,.key,application/x-pem-file'
					}
				]
			: [])
	]);

	async function readPem(field: PemField, file: File | undefined) {
		if (!file) return;
		const contents = await file.text();
		// Tetra takes PEM contents; a DER file would read as garbage text.
		if (!contents.trimStart().startsWith('-----BEGIN ')) {
			pemErrors[field] = t('network-8021x-not-pem');
			return;
		}
		pemErrors[field] = undefined;
		settings[field] = contents;
		settings[`${field}Name`] = file.name;
	}
</script>

<label class="field-label" for="{idPrefix}-eap"
	>{t('network-wifi-eap')}<select id="{idPrefix}-eap" bind:value={settings.eap}>
		{#each eapMethods as method}<option value={method}>{method.toUpperCase()}</option>{/each}
	</select></label
>
{#if isTunneled(settings.eap)}
	<label class="field-label" for="{idPrefix}-phase2"
		>{t('network-wifi-phase2')}<select id="{idPrefix}-phase2" bind:value={settings.phase2}>
			{#each phase2Methods as method}<option value={method}>{method.toUpperCase()}</option>{/each}
		</select></label
	>
{/if}
<label class="field-label" for="{idPrefix}-identity"
	>{t('network-8021x-identity')}<input
		id="{idPrefix}-identity"
		bind:value={settings.identity}
		autocomplete="username"
	/></label
>
{#if isTunneled(settings.eap)}
	<label class="field-label" for="{idPrefix}-anonymous-identity"
		>{t('network-wifi-anonymous-identity')}<input
			id="{idPrefix}-anonymous-identity"
			bind:value={settings.anonymousIdentity}
		/></label
	>
{/if}
{#if settings.eap !== 'tls'}
	<label class="field-label" for="{idPrefix}-password"
		>{t('network-wifi-password')}<input
			id="{idPrefix}-password"
			type="password"
			bind:value={settings.password}
			autocomplete="current-password"
		/></label
	>
{/if}
<label class="field-label" for="{idPrefix}-domain"
	>{t('network-8021x-domain')}<input
		id="{idPrefix}-domain"
		bind:value={settings.domain}
		placeholder="example.com"
		aria-describedby="{idPrefix}-domain-help"
	/></label
>
<p class="field-help" id="{idPrefix}-domain-help">{t('network-8021x-domain-help')}</p>
{#each pemUploads as upload (upload.field)}
	<div class="certificate-upload-row">
		<span class="certificate-upload-label">{upload.label}</span>
		<span class="certificate-upload-name"
			>{settings[`${upload.field}Name`] || t('network-wifi-no-certificate')}</span
		>
		<label class="certificate-browse" for="{idPrefix}-{upload.field}-upload"
			>{t('network-wifi-browse')}</label
		>
		<input
			id="{idPrefix}-{upload.field}-upload"
			class="certificate-file-input"
			type="file"
			accept={upload.accept}
			onchange={(event) => readPem(upload.field, event.currentTarget.files?.[0])}
		/>
	</div>
	{#if pemErrors[upload.field]}<p class="field-error" role="alert">
			{pemErrors[upload.field]}
		</p>{/if}
{/each}
{#if settings.eap === 'tls'}
	<label class="field-label" for="{idPrefix}-private-key-password"
		>{t('network-8021x-private-key-password')}<input
			id="{idPrefix}-private-key-password"
			type="password"
			bind:value={settings.privateKeyPassword}
			autocomplete="off"
		/></label
	>
{/if}
