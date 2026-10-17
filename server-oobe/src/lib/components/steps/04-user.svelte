<script lang="ts">
	import StepLayout from '$lib/components/StepLayout.svelte';
	import { t } from '$lib/i18n.svelte';
	let fullName = $state('');
	let usernameEdited = $state(false);
	let touched = $state(false);
	let { username = $bindable(''), onBack, onContinue } = $props();
	let usernameError = $derived(
		touched && !/^[a-z_][a-z0-9_-]*$/.test(username) ? t('username-error') : ''
	);
	function updateName(value: string) {
		fullName = value;
		if (!usernameEdited)
			username =
				value
					.trim()
					.split(/\s+/)[0]
					?.toLowerCase()
					.replace(/[^a-z0-9_]/g, '') ?? '';
	}
</script>

<StepLayout
	title={t('administrator-title')}
	{onBack}
	{onContinue}
	canContinue={Boolean(username) && !usernameError}
>
	<label class="field-label" for="full-name"
		>{t('full-name-label')}<input
			class="text-input"
			id="full-name"
			value={fullName}
			oninput={(event) => updateName(event.currentTarget.value)}
		/></label
	>
	<label class="field-label" for="username"
		>{t('username-label')}<input
			class="text-input"
			id="username"
			bind:value={username}
			oninput={() => {
				usernameEdited = true;
				touched = true;
			}}
			aria-describedby="username-error"
		/></label
	>
	{#if usernameError}<p class="field-error" id="username-error">{usernameError}</p>{/if}
</StepLayout>
