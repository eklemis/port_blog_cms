// Public API of the i18n slice.
export {
	BASE_LOCALE,
	LOCALES,
	LOCALE_COOKIE,
	availableLocales,
	isLocale,
	resolveLocale,
	type Locale
} from './locale';
export { catalogues, en, type Catalogue } from './messages';
export { messagesFor } from './messages.svelte';
