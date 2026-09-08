import { expect, test } from 'vitest';
import { SLUG_MAX, slugError, slugFrom } from './slug';

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
	// Trimmed, non-empty, at most 200 — VALIDATION.md. Rust counts chars, so
	// an emoji is one, not two.
	expect(slugError('a'.repeat(SLUG_MAX))).toBeUndefined();
	expect(slugError('a'.repeat(SLUG_MAX + 1))).toBeDefined();
	expect(slugError('🌱'.repeat(SLUG_MAX))).toBeUndefined();
});

test('nothing else is rejected, because the server rejects nothing else', () => {
	// It trims and lowercases and asks for non-empty. Inventing a character
	// rule here would refuse addresses the API would have accepted.
	expect(slugError('Mixed Case With Spaces')).toBeUndefined();
	expect(slugError('ünïcode-slug')).toBeUndefined();
});
