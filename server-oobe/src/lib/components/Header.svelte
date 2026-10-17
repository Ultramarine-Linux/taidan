<script lang="ts">
	import moonIcon from '$lib/icons/moon.svg';
	import sunIcon from '$lib/icons/sun.svg';
	import { t } from '$lib/i18n.svelte';

	let { theme, onToggleTheme }: { theme: 'light' | 'dark'; onToggleTheme: () => void } = $props();
</script>

<header class="topbar">
	<img class="brand-logo" src="/ultramarine-logo.svg" alt="" />
	<div class="brand-copy">
		<p class="eyebrow">{t('brand-name')}</p>
		<h1>{t('setup-title')}</h1>
	</div>
	<button
		class="theme-toggle"
		type="button"
		onclick={onToggleTheme}
		aria-label={theme === 'light' ? t('switch-to-dark') : t('switch-to-light')}
		title={theme === 'light' ? t('switch-to-dark') : t('switch-to-light')}
	>
		<img class="theme-icon theme-icon-moon" src={moonIcon} alt="" aria-hidden="true" />
		<img class="theme-icon theme-icon-sun" src={sunIcon} alt="" aria-hidden="true" />
	</button>
</header>
