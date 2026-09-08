import { expect, test } from 'vitest';
import { EMAIL_INVALID, emailError, normaliseEmail } from './email';

/**
 * The email rule, shared by every screen that takes an address.
 */

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

test('normalising trims but does not lowercase — the server does that', () => {
	expect(normaliseEmail('  Jane@Example.com ')).toBe('Jane@Example.com');
});
