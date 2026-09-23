import { availableLocales } from '$lib/i18n.svelte';

export type Language = {
	locale: string;
	name: string;
	nativeName: string;
};

const languageNames: Record<string, [string, string]> = {
	'en-US': ['English', 'English'],
	cs: ['Czech', 'Čeština'],
	es: ['Spanish', 'Español'],
	fi: ['Finnish', 'Suomi'],
	fr: ['French', 'Français'],
	hi: ['Hindi', 'हिन्दी'],
	ja: ['Japanese', '日本語'],
	ka: ['Georgian', 'ქართული'],
	pl: ['Polish', 'Polski'],
	sv: ['Swedish', 'Svenska'],
	th: ['Thai', 'ไทย'],
	vi: ['Vietnamese', 'Tiếng Việt'],
	'zh-Hans': ['Chinese (Simplified)', '简体中文'],
	'zh-Hant': ['Chinese (Traditional)', '繁體中文']
};

export const supportedLanguages: Language[] = availableLocales.map((locale) => {
	const [name, nativeName] = languageNames[locale] ?? [locale, locale];
	return { locale, name, nativeName };
});
