import { BASE_LOCALE, type Locale } from './locale';
import { catalogues, en, type Catalogue } from './messages';

/**
 * The copy for a locale, with English behind it.
 *
 * A missing catalogue falls back rather than throwing: a half-translated build
 * should show an English sentence, not a blank space or a key.
 */
export function messagesFor(locale: Locale): Catalogue {
	return catalogues[locale] ?? catalogues[BASE_LOCALE] ?? en;
}
