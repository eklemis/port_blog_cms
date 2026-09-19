import { expect, test } from 'vitest';
import {
	BASE_LOCALE,
	LOCALE_COOKIE,
	availableLocales,
	isLocale,
	localeForRoute,
	resolveLocale
} from './locale';

/**
 * Both languages, so the matching rules can be proven before Indonesian copy
 * exists. What this build actually offers is asserted separately, below.
 */
const both = { offered: ['en', 'id'] as const };

/**
 * Career Studio §02 — the UI language.
 *
 * Three states, like the theme in §01: a locale chosen explicitly, or none,
 * in which case the browser decides. Stored per browser first, because the
 * auth and public shells have no session to write to.
 */

test('an explicit choice wins over everything', () => {
	expect(resolveLocale({ stored: 'id', header: 'en-GB,en;q=0.9', ...both })).toBe('id');
});

test('with no choice, the browser decides', () => {
	// The third state. Not a default pretending to be a preference.
	expect(resolveLocale({ stored: null, header: 'id-ID,id;q=0.9,en;q=0.8', ...both })).toBe('id');
});

test('a browser asking for something we do not have gets the base locale', () => {
	expect(resolveLocale({ stored: null, header: 'fr-FR,fr;q=0.9', ...both })).toBe('en');
	expect(resolveLocale({ stored: null, header: null, ...both })).toBe('en');
});

test('a stored value that is not a locale is not trusted', () => {
	// It arrives from a cookie, which is to say from anybody.
	expect(resolveLocale({ stored: 'fr', header: 'id-ID', ...both })).toBe('id');
	expect(resolveLocale({ stored: '../../etc', header: null, ...both })).toBe('en');
});

test('matches on the language, not the region', () => {
	expect(resolveLocale({ stored: null, header: 'en-AU', ...both })).toBe('en');
	expect(resolveLocale({ stored: null, header: 'id-ID', ...both })).toBe('id');
});

test('quality values are respected, not just position', () => {
	expect(resolveLocale({ stored: null, header: 'en;q=0.2, id;q=0.9', ...both })).toBe('id');
});

test('a language with no copy behind it is never resolved to', () => {
	// The switcher reads the same list. Offering Indonesian before the copy
	// exists would change the label and nothing else.
	expect(resolveLocale({ stored: 'id', header: 'id-ID' })).toBe(BASE_LOCALE);
});

test('only locales with a catalogue behind them are on offer', () => {
	// §02: "build the switcher so a third locale is a file rather than a
	// refactor". `availableLocales` is that file list, and the switcher reads
	// it rather than a hard-coded pair — so it cannot offer a language the
	// product cannot speak.
	const offered = availableLocales();

	expect(offered.length).toBeGreaterThan(0);
	expect(offered.every((locale) => isLocale(locale))).toBe(true);
});

test('the cookie is named once, so the server and the browser agree', () => {
	expect(LOCALE_COOKIE).toBe('arch_locale');
});

/**
 * Ruling A, 20 September: public pages are not localised. No segment, no
 * negotiation, URLs unchanged.
 *
 * The reasoning is not that localising them is hard. It is that localising a
 * public page translates the chrome around an article the product cannot
 * translate and does not claim to — content language is per document and never
 * retranslates, so a localised shell over an untranslated body is a promise the
 * page cannot keep.
 *
 * These assert with both locales offered on purpose: with only English built,
 * this rule and its opposite produce identical output, and a test that cannot
 * fail is not a test.
 */

test('a public page is never localised, whatever the reader prefers', () => {
	expect(localeForRoute({ routeId: '/[username]', stored: 'id', header: 'id-ID', ...both })).toBe(
		'en'
	);
	expect(
		localeForRoute({ routeId: '/[username]/blog/[slug]', stored: 'id', header: 'id-ID', ...both })
	).toBe('en');
});

test('the console and the auth shell still negotiate — the pill keeps its 30 places', () => {
	expect(localeForRoute({ routeId: '/studio/posts', stored: 'id', header: null, ...both })).toBe(
		'id'
	);
	expect(localeForRoute({ routeId: '/auth/login', stored: null, header: 'id-ID', ...both })).toBe(
		'id'
	);
});

test('the public shell is matched by segment, not by prefix', () => {
	// `/[username]-archive` is not a public page, and a startsWith that missed
	// this would quietly refuse to localise any route beginning with the same
	// characters.
	expect(
		localeForRoute({ routeId: '/[username]-archive', stored: 'id', header: null, ...both })
	).toBe('id');
});

test('an unmatched route is treated as console, not as public', () => {
	// `null` is a 404. Guessing "public" would make every unknown path refuse to
	// localise, which is the wrong way round: the ruling names five routes, it
	// does not name everything else.
	expect(localeForRoute({ routeId: null, stored: 'id', header: null, ...both })).toBe('id');
});
