import { expect, test } from 'vitest';
import { SLUG_CHARACTERS, SLUG_MAX, SLUG_TOO_LONG, slugError, slugFrom } from './slug';

/**
 * The public address a post or a project will live at.
 *
 * Shared rather than owned by posts: J5 says projects handle slugs "the same
 * as posts", and two entities may not import each other.
 */

test('a title becomes the address someone would have typed', () => {
	expect(slugFrom('Building a CMS in Rust')).toBe('building-a-cms-in-rust');
});

test('punctuation and repeats collapse rather than surviving', () => {
	expect(slugFrom('Why hexagonal, actually?')).toBe('why-hexagonal-actually');
	expect(slugFrom('One  --  two')).toBe('one-two');
});

test('accents are folded, not dropped', () => {
	// "Résumés" must not become "rsums". The product's own nav says Résumés.
	expect(slugFrom('Résumés and CVs')).toBe('resumes-and-cvs');
});

test('it never starts or ends with a separator', () => {
	expect(slugFrom('  Hello!  ')).toBe('hello');
	expect(slugFrom('— dash first')).toBe('dash-first');
});

test('a title with nothing sluggable gives nothing, rather than a dash', () => {
	// The field then shows as empty and the person writes their own, which is
	// better than an address that reads "-".
	expect(slugFrom('!!!')).toBe('');
	expect(slugFrom('')).toBe('');
});

test('it is capped where the column is', () => {
	expect(slugFrom('word '.repeat(100)).length).toBeLessThanOrEqual(SLUG_MAX);
	expect(slugFrom('word '.repeat(100)).endsWith('-')).toBe(false);
});

// ── what makes one unusable ────────────────────────────────────────────────

test('an address is required, because the public URL is built from it', () => {
	expect(slugError('')).toBeDefined();
	expect(slugError('   ')).toBeDefined();
});

test('the length rule is the server’s, counted the way the server counts', () => {
	// Trimmed, non-empty, at most 200. Rust counts `chars()`, so an emoji is
	// one character and not the two UTF-16 units JavaScript stores it in.
	expect(slugError('a'.repeat(SLUG_MAX))).toBeUndefined();
	expect(slugError('a'.repeat(SLUG_MAX + 1))).toBe(SLUG_TOO_LONG);

	// 200 emoji is 200 characters and 400 units. Counting units would call
	// this too long; counting characters lets it through to the rule it
	// actually breaks, which is the alphabet.
	expect(slugError('🌱'.repeat(SLUG_MAX))).toBe(SLUG_CHARACTERS);
});

test('a slug is a-z, 0-9 and hyphen, and nothing else', () => {
	// The rule was missing from VALIDATION.md when this was written, so the
	// first cut of it accepted anything the doc did not forbid. Both services
	// test `is_ascii_alphanumeric() || c == '-'`, so a space is an
	// INVALID_SLUG rather than an address with %20 in it.
	expect(slugError('Mixed Case With Spaces')).toBeDefined();
	expect(slugError('what?')).toBeDefined();
	expect(slugError('under_score')).toBeDefined();
	expect(slugError('building-a-cms-2')).toBeUndefined();
});

test('ascii means ascii, so an accented address is refused too', () => {
	// `café` and `日本語` are good titles and impossible slugs. Deriving one
	// folds the accents; typing one by hand has to be refused, because the
	// server will refuse it.
	expect(slugError('café')).toBeDefined();
	expect(slugError('日本語')).toBeDefined();
});

test('the trimmed value is what is judged, since the server trims first', () => {
	expect(slugError('  building-a-cms  ')).toBeUndefined();
});
