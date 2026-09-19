import { catalogues } from './messages';

/**
 * The UI language — Career Studio §02.
 *
 * Two settings hide behind the phrase "language selection", and this is the
 * first: what the navigation, labels, buttons, errors, dates and number formats
 * are written in. One per person. The second — what a CV or cover letter is
 * *written* in — lives on the document and is not this.
 *
 * Three states, exactly like the theme in §01. A locale chosen explicitly, or
 * none at all, in which case the browser decides. "No choice" is a real state
 * and not a default wearing its clothes: someone who has never touched the
 * switcher should get Indonesian in Jakarta without asking for it.
 *
 * Stored per browser first. The auth and public shells have no session to write
 * to, so the cookie is the source of truth everywhere and the account catches
 * up at sign-in.
 */

/** English at launch, Indonesian beside it. §02: a third is a file, not a refactor. */
export const LOCALES = ['en', 'id'] as const;

export type Locale = (typeof LOCALES)[number];

/** The base. Not a preference — what is left when nobody has expressed one. */
export const BASE_LOCALE: Locale = 'en';

/**
 * One name for the cookie, so the server that reads it and the browser that
 * writes it cannot drift apart.
 */
export const LOCALE_COOKIE = 'arch_locale';

export function isLocale(value: unknown): value is Locale {
	return typeof value === 'string' && (LOCALES as readonly string[]).includes(value);
}

/**
 * The locales this build can actually speak.
 *
 * Derived from which catalogues exist, never from `LOCALES` — a language with
 * no copy behind it is a language the product cannot speak, and offering it
 * would be a switcher that changes the label and nothing else.
 */
export function availableLocales(): Locale[] {
	return LOCALES.filter((locale) => catalogues[locale] !== undefined);
}

/** `Accept-Language`, best first, honouring q-values rather than order alone. */
function preferredFrom(header: string): string[] {
	return header
		.split(',')
		.map((part) => {
			const [tag, ...params] = part.trim().split(';');
			const q = params.find((p) => p.trim().startsWith('q='));
			return { tag: tag.trim().toLowerCase(), q: q ? Number(q.split('=')[1]) : 1 };
		})
		.filter((entry) => entry.tag && Number.isFinite(entry.q))
		.sort((a, b) => b.q - a.q)
		.map((entry) => entry.tag);
}

/**
 * Which language to render in.
 *
 * `stored` comes from a cookie, which is to say from anybody, so it is checked
 * rather than trusted. Matching is on the language subtag: `en-AU` is English,
 * and a region we have no copy for is not a reason to fall back to another
 * language entirely.
 */
export function resolveLocale({
	stored,
	header,
	offered = availableLocales()
}: {
	stored?: string | null;
	header?: string | null;
	/**
	 * Which locales are on offer. Defaults to the catalogues this build has —
	 * injectable so the matching rules can be tested against a second language
	 * before one ships, rather than only after.
	 */
	offered?: readonly Locale[];
}): Locale {
	if (isLocale(stored) && offered.includes(stored)) return stored;

	for (const tag of preferredFrom(header ?? '')) {
		const language = tag.split('-')[0];
		const match = offered.find((locale) => locale === language);
		if (match) return match;
	}

	return BASE_LOCALE;
}
