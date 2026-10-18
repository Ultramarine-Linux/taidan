<script lang="ts">
	import StepLayout from '$lib/components/StepLayout.svelte';
	import { t } from '$lib/i18n.svelte';
	import type { UpdatePolicy } from '$lib/oobe-state';
	let {
		policy = $bindable<UpdatePolicy>('automatic'),
		onBack,
		onContinue
	}: {
		policy?: UpdatePolicy;
		onBack: () => void;
		onContinue: () => void;
	} = $props();
</script>

<StepLayout title={t('updates-title')} {onBack} {onContinue}>
	<p class="lead">{t('updates-description')}</p>
	<section class="form-section">
		<fieldset>
			<legend>{t('updates-policy')}</legend>
			<label class="toggle-option" for="updates-automatic">
				<span>
					<strong>{t('updates-automatic')}</strong>
					<small>{t('updates-automatic-description')}</small>
				</span>
				<input
					id="updates-automatic"
					type="radio"
					name="update-policy"
					value="automatic"
					bind:group={policy}
				/>
			</label>
			<label class="toggle-option" for="updates-notify">
				<span>
					<strong>{t('updates-notify')}</strong>
					<small>{t('updates-notify-description')}</small>
				</span>
				<input
					id="updates-notify"
					type="radio"
					name="update-policy"
					value="notify"
					bind:group={policy}
				/>
			</label>
		</fieldset>
	</section>
</StepLayout>

<style>
	.lead {
		margin-bottom: 1rem;
		color: var(--muted-foreground);
		line-height: 1.5;
	}
</style>
