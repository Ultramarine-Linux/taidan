import { FluentBundle, FluentResource } from '@fluent/bundle';

const catalogSources = import.meta.glob('../../../po/*/server-oobe.ftl', {
	eager: true,
	query: '?raw',
	import: 'default'
}) as Record<string, string>;

const catalogs = new Map<string, FluentBundle>();
for (const [path, source] of Object.entries(catalogSources)) {
	const locale = path.split('/').at(-2);
	if (!locale) continue;
	const bundle = new FluentBundle(locale);
	bundle.addResource(new FluentResource(source));
	catalogs.set(locale, bundle);
}

export const availableLocales = [...catalogs.keys()].sort();

export const localeState = $state({
	value: catalogs.has('en-US') ? 'en-US' : (availableLocales[0] ?? 'en-US')
});

export function setLocale(locale: string) {
	if (catalogs.has(locale)) localeState.value = locale;
}

export function t(id: string, args?: Record<string, string | number>): string {
	const bundle = catalogs.get(localeState.value) ?? catalogs.get('en-US');
	if (!bundle) return id;
	const message = bundle.getMessage(id);
	if (!message?.value) return id;
	const errors: Error[] = [];
	return bundle.formatPattern(message.value, args, errors) || id;
}

export function hasLocale(locale: string): boolean {
	return catalogs.has(locale);
}

export function detectLocale(): void {
	if (typeof navigator === 'undefined') return;
	const candidates = [navigator.language, ...(navigator.languages || [])];
	for (const candidate of candidates) {
		if (catalogs.has(candidate)) {
			localeState.value = candidate;
			return;
		}
		const langCode = candidate.split('-')[0];
		if (catalogs.has(langCode)) {
			localeState.value = langCode;
			return;
		}
	}
}
