<script lang="ts">
	import StepLayout from '$lib/components/StepLayout.svelte';
	import { t } from '$lib/i18n.svelte';
	let publicKey = $state('');
	let fileName = $state('');
	let { value = $bindable(''), onBack, onContinue }: { value?: string; onBack: () => void; onContinue: () => void } = $props();

	function update(valueToSet: string, name = '') {
		publicKey = valueToSet.trim();
		fileName = name;
		value = publicKey;
	}
</script>

<StepLayout title={t('ssh-key-title')} {onBack} {onContinue} canContinue={publicKey.length === 0 || publicKey.startsWith('ssh-') || publicKey.startsWith('ecdsa-') || publicKey.startsWith('ed25519-')}>
	<p class="lead">{t('ssh-key-description')}</p>
	<label class="field-label" for="ssh-public-key">{t('ssh-key-label')}<textarea id="ssh-public-key" bind:value={publicKey} oninput={() => update(publicKey)} placeholder="ssh-ed25519 AAAA… user@host"></textarea></label>
	<div class="ssh-upload-row">
		<span>{fileName || t('ssh-key-no-file')}</span>
		<label class="certificate-browse" for="ssh-key-file">{t('network-wifi-browse')}</label>
		<input id="ssh-key-file" class="certificate-file-input" type="file" accept=".pub,text/plain" onchange={async (event) => { const file = event.currentTarget.files?.[0]; if (file) update(await file.text(), file.name); }} />
	</div>
</StepLayout>

<style>
	.lead { color: var(--muted-foreground); }
	textarea { width: 100%; min-height: 7rem; resize: vertical; border: 1px solid var(--border); background: var(--background); color: var(--foreground); padding: 0.7rem; font: inherit; }
	.ssh-upload-row { display: flex; align-items: center; justify-content: space-between; gap: 1rem; border: 1px dashed var(--border); padding: 0.5rem 0.7rem; color: var(--muted-foreground); font-size: 0.8rem; }
</style>
