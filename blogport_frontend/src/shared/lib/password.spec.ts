import { expect, test } from 'vitest';
import { PASSWORD_MAX, PASSWORD_TOO_SHORT, passwordError } from './password';

/**
 * The password rule, shared by sign-in and register. Mirrored from Frontend
 * Handoff §03 so a field-level error resolves without a round trip — and no
 * further, because a client rule the server does not share rejects input the
 * backend would have accepted.
 */

// ── password ───────────────────────────────────────────────────────────────

test('twelve characters is enough', () => {
	expect(passwordError('abcdefghijkl')).toBeUndefined();
});

test('eleven characters is not', () => {
	expect(passwordError('abcdefghijk')).toBe(PASSWORD_TOO_SHORT);
});

test('an empty password reports the same requirement, not a separate one', () => {
	// One message per field: "At least 12 characters." is the requirement, and
	// nothing typed is simply the shortest way to miss it.
	expect(passwordError('')).toBe(PASSWORD_TOO_SHORT);
});

test('there is no complexity rule — length only', () => {
	// BasicPasswordPolicy checks nothing but 12 <= length <= 128. The OpenAPI
	// example is `SecurePass123!`, which implies uppercase/digit/symbol rules
	// that do not exist. A client rule the server lacks rejects passwords the
	// backend would have accepted. Frontend Handoff §03.
	expect(passwordError('aaaaaaaaaaaa')).toBeUndefined();
	expect(passwordError('all lowercase words here')).toBeUndefined();
	expect(passwordError('!!!!!!!!!!!!')).toBeUndefined();
	expect(passwordError('日本語のパスワードですよ')).toBeUndefined();
});

test('the maximum is 128, and the field is capped rather than scolded', () => {
	expect(passwordError('a'.repeat(PASSWORD_MAX))).toBeUndefined();
	expect(PASSWORD_MAX).toBe(128);
});

test('length is counted in characters, the way the server counts it', () => {
	// The backend measures with Rust's `chars()`; JS `.length` measures UTF-16
	// code units, so twelve astral characters look like twenty-four here and a
	// password of eleven would look long enough. Count code points.
	expect(passwordError('👍'.repeat(11))).toBe(PASSWORD_TOO_SHORT);
	expect(passwordError('👍'.repeat(12))).toBeUndefined();
});

test('whitespace inside a password is part of it', () => {
	// Passwords are not trimmed anywhere — normalisation for password is "none".
	expect(passwordError('  ten chars  ')).toBeUndefined();
});
