<script lang="ts">
	import StepLayout from '$lib/components/StepLayout.svelte';
	import { t } from '$lib/i18n.svelte';
	import type { RemoteAccessForm } from '$lib/oobe-state';
	let {
		form = $bindable<RemoteAccessForm>(),
		onBack,
		onContinue
	}: {
		form: RemoteAccessForm;
		onBack: () => void;
		onContinue: () => void;
	} = $props();

	let canContinue = $derived(
		(!form.tailscale ||
			(form.tailscaleAuthKey.trim().length > 0 &&
				(!form.tailscaleFunnel || (form.tailscaleServe && form.funnelConfirmed)))) &&
			(!form.cloudflared || form.cloudflaredToken.trim().length > 0)
	);

	$effect(() => {
		// Funnel publishes what Serve shares; without Serve there is nothing to publish.
		if (!form.tailscaleServe) form.tailscaleFunnel = false;
		if (!form.tailscaleFunnel) form.funnelConfirmed = false;
	});
</script>

<StepLayout title={t('remote-access-title')} {onBack} {onContinue} {canContinue}>
	<p class="lead">{t('remote-access-description')}</p>

	<section class="form-section">
		<label class="toggle-option" for="tailscale-enable">
			<span>
				<strong>{t('tailscale-enable')}</strong>
				<small>{t('tailscale-enable-description')}</small>
			</span>
			<input id="tailscale-enable" type="checkbox" bind:checked={form.tailscale} />
		</label>
		{#if form.tailscale}
			<label class="field-label" for="tailscale-auth-key">
				{t('tailscale-auth-key')}
				<input
					id="tailscale-auth-key"
					type="password"
					autocomplete="off"
					spellcheck="false"
					placeholder="tskey-auth-…"
					bind:value={form.tailscaleAuthKey}
					aria-describedby="tailscale-auth-key-description"
				/>
				<small id="tailscale-auth-key-description">{t('tailscale-auth-key-description')}</small>
			</label>
			<label class="toggle-option" for="tailscale-serve">
				<span>
					<strong>{t('tailscale-serve')}</strong>
					<small>{t('tailscale-serve-description')}</small>
				</span>
				<input id="tailscale-serve" type="checkbox" bind:checked={form.tailscaleServe} />
			</label>
			<label class="toggle-option" for="tailscale-funnel">
				<span>
					<strong>{t('tailscale-funnel')}</strong>
					<small>{t('tailscale-funnel-description')}</small>
				</span>
				<input
					id="tailscale-funnel"
					type="checkbox"
					disabled={!form.tailscaleServe}
					bind:checked={form.tailscaleFunnel}
				/>
			</label>
			{#if form.tailscaleFunnel}
				<div class="panel error-panel" role="alert">
					<div class="status-icon">!</div>
					<div>
						<h3>{t('tailscale-funnel-warning-title')}</h3>
						<p>{t('tailscale-funnel-warning')}</p>
						<label class="confirm" for="tailscale-funnel-confirm">
							<input
								id="tailscale-funnel-confirm"
								type="checkbox"
								bind:checked={form.funnelConfirmed}
							/>
							{t('tailscale-funnel-confirm')}
						</label>
					</div>
				</div>
			{/if}
		{/if}
	</section>

	<section class="form-section">
		<label class="toggle-option" for="cloudflared-enable">
			<span>
				<strong>{t('cloudflared-enable')}</strong>
				<small>{t('cloudflared-enable-description')}</small>
			</span>
			<input id="cloudflared-enable" type="checkbox" bind:checked={form.cloudflared} />
		</label>
		{#if form.cloudflared}
			<label class="field-label" for="cloudflared-token">
				{t('cloudflared-token')}
				<input
					id="cloudflared-token"
					type="password"
					autocomplete="off"
					spellcheck="false"
					bind:value={form.cloudflaredToken}
					aria-describedby="cloudflared-token-description"
				/>
				<small id="cloudflared-token-description">{t('cloudflared-token-description')}</small>
			</label>
		{/if}
	</section>
</StepLayout>

<style>
	.lead {
		margin-bottom: 1rem;
		color: var(--muted-foreground);
		line-height: 1.5;
	}

	.form-section + .form-section {
		margin-top: 1rem;
	}

	.field-label small {
		color: var(--muted-foreground);
		font-size: 0.75rem;
		font-weight: 400;
		line-height: 1.4;
	}

	.error-panel {
		margin: 0;
	}

	.error-panel h3 {
		margin: 0;
	}

	.confirm {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		margin-top: 0.5rem;
		color: var(--foreground);
		font-size: 0.85rem;
		font-weight: 600;
	}
</style>
