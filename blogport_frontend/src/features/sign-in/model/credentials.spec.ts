import { expect, test } from 'vitest';
import {
	EMAIL_INVALID,
	PASSWORD_MAX,
	PASSWORD_TOO_SHORT,
	emailError,
	passwordError
} from './credentials';

/**
 * Sign-in field rules. Mirrored from Frontend Handoff §03 so a field-level
 * error resolves without a round trip — and no further, because a client rule
 * the server does not share rejects input the backend would have accepted.
 */

// ── email ──────────────────────────────────────────────────────────────────

test('accepts an ordinary address', () => {
	expect(emailError('jane@example.com')).toBeUndefined();
});

test('accepts an address with a plus tag and a multi-part domain', () => {
	expect(emailError('jane+blog@mail.example.co.uk')).toBeUndefined();
});

test('rejects half a typed address', () => {
	expect(emailError('jane@')).toBe(EMAIL_INVALID);
	expect(emailError('jane')).toBe(EMAIL_INVALID);
});

test('an empty field is not an address either', () => {
	expect(emailError('')).toBe(EMAIL_INVALID);
});

test('surrounding whitespace is not the person’s mistake', () => {
	// The backend trims before validating; a pasted address with a trailing
	// space must not be called invalid.
	expect(emailError('  jane@example.com  ')).toBeUndefined();
});

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
