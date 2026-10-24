import { MediaQuery } from 'svelte/reactivity';

/**
 * Theme and high contrast, matching the Dashboard: dark unless the user picks
 * light, and high contrast following `prefers-contrast: more` unless the user
 * turns it on or off. Reflected as the `dark` class and `data-contrast="more"`
 * on <html>. The inline script in src/app.html resolves both before first
 * paint; keep it in sync with this file.
 */
export type Theme = 'light' | 'dark';
export type ContrastPreference = 'auto' | 'on' | 'off';

export const themeStorageKey = 'oobe-theme';
export const contrastStorageKey = 'oobe-contrast';

function read(key: string): string | null {
	if (typeof localStorage === 'undefined') return null;
	try {
		return localStorage.getItem(key);
	} catch {
		return null;
	}
}

function write(key: string, value: string) {
	try {
		localStorage.setItem(key, value);
	} catch {
		// Storage unavailable; the choice lasts for this page.
	}
}

function isContrastPreference(value: unknown): value is ContrastPreference {
	return value === 'auto' || value === 'on' || value === 'off';
}

class Appearance {
	readonly #systemContrast = new MediaQuery('prefers-contrast: more', false);

	theme = $state<Theme>('dark');
	contrastPreference = $state<ContrastPreference>('auto');
	#loaded = false;

	/** Load the stored choices. Call once in the browser, before `apply`. */
	load() {
		this.#loaded = true;
		this.theme = read(themeStorageKey) === 'light' ? 'light' : 'dark';
		const contrast = read(contrastStorageKey);
		this.contrastPreference = isContrastPreference(contrast) ? contrast : 'auto';
	}

	get highContrast() {
		return (
			this.contrastPreference === 'on' ||
			(this.contrastPreference === 'auto' && this.#systemContrast.current)
		);
	}

	toggleTheme() {
		this.theme = this.theme === 'light' ? 'dark' : 'light';
		write(themeStorageKey, this.theme);
	}

	toggleContrast() {
		this.contrastPreference = this.highContrast ? 'off' : 'on';
		write(contrastStorageKey, this.contrastPreference);
	}

	/** Reflect the current choices onto the document. Call from an effect. */
	apply() {
		// Read both first so the calling effect tracks them.
		const dark = this.theme === 'dark';
		const more = this.highContrast;
		// Until the stored choices are read, keep what app.html already set.
		if (!this.#loaded) return;
		const root = document.documentElement;
		root.classList.toggle('dark', dark);
		if (more) root.setAttribute('data-contrast', 'more');
		else root.removeAttribute('data-contrast');
	}
}

export const appearance = new Appearance();
