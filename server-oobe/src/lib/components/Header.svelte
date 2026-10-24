<script lang="ts">
	import Icon from './Icon.svelte';
	import { appearance } from '$lib/appearance.svelte';
	import { t } from '$lib/i18n.svelte';

	let {
		stepLabel,
		onOpenNavigation
	}: {
		/** The current step, shown as the last breadcrumb. */
		stepLabel: string;
		onOpenNavigation: () => void;
	} = $props();

	let themeLabel = $derived(
		appearance.theme === 'light' ? t('switch-to-dark') : t('switch-to-light')
	);
</script>

<header class="topbar">
	<div class="topbar-start">
		<button
			class="icon-button borderless menu-button"
			type="button"
			onclick={onOpenNavigation}
			aria-label={t('open-steps')}
			aria-haspopup="dialog"
		>
			<Icon name="menu" />
		</button>
		<span class="topbar-logo">
			<img class="brand-logo" src="/ultramarine-logo.svg" alt="" />
		</span>
		<h1 class="breadcrumb">
			<span class="breadcrumb-root">{t('setup-title')}</span>
			<Icon name="chevron-right" class="breadcrumb-separator" />
			<span class="breadcrumb-current">{stepLabel}</span>
		</h1>
	</div>
	<div class="topbar-end">
		<button
			class="icon-button"
			type="button"
			onclick={() => appearance.toggleContrast()}
			aria-pressed={appearance.highContrast}
			aria-label={t('high-contrast')}
			title={t('high-contrast')}
		>
			<Icon name="contrast" />
		</button>
		<button
			class="icon-button"
			type="button"
			onclick={() => appearance.toggleTheme()}
			aria-label={themeLabel}
			title={themeLabel}
		>
			<Icon name={appearance.theme === 'light' ? 'moon' : 'sun'} />
		</button>
	</div>
</header>
