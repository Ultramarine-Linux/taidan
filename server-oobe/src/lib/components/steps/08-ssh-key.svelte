<script lang="ts">
	import StepLayout from '$lib/components/StepLayout.svelte';
	import { t } from '$lib/i18n.svelte';
	let publicKey = $state('');
	let fileName = $state('');
	let {
		value = $bindable(''),
		onBack,
		onContinue
	}: { value?: string; onBack: () => void; onContinue: () => void } = $props();

	let valid = $derived(
		publicKey.length === 0 ||
			publicKey.startsWith('ssh-') ||
			publicKey.startsWith('ecdsa-') ||
			publicKey.startsWith('ed25519-')
	);

	function update(valueToSet: string, name = '') {
		publicKey = valueToSet.trim();
		fileName = name;
		value = publicKey;
	}
</script>

<StepLayout
	title={t('ssh-key-title')}
	description={t('ssh-key-description')}
	{onBack}
	{onContinue}
	canContinue={valid}
>
	<label class="field-label" for="ssh-public-key"
		>{t('ssh-key-label')}<textarea
			id="ssh-public-key"
			class="mono-textarea"
			bind:value={publicKey}
			oninput={() => update(publicKey)}
			placeholder="ssh-ed25519 AAAA… user@host"
			spellcheck="false"
			aria-invalid={valid ? undefined : 'true'}></textarea></label
	>
	<div class="ssh-upload-row">
		<span>{fileName || t('ssh-key-no-file')}</span>
		<label class="certificate-browse" for="ssh-key-file">{t('network-wifi-browse')}</label>
		<input
			id="ssh-key-file"
			class="certificate-file-input"
			type="file"
			accept=".pub,text/plain"
			onchange={async (event) => {
				const file = event.currentTarget.files?.[0];
				if (file) update(await file.text(), file.name);
			}}
		/>
	</div>
</StepLayout>

<style>
	.mono-textarea {
		font-family: var(--font-mono) !important;
		font-size: 0.8125rem !important;
	}
</style>
