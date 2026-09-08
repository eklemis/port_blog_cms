import { expect, test } from 'vitest';
import {
	FULL_NAME_REQUIRED,
	USERNAME_INVALID,
	USERNAME_TOO_SHORT,
	fullNameError,
	publicAddress,
	usernameError
} from './fields';

/**
 * The two rules register adds. Email and password are shared with sign-in and
 * live in shared/lib.
 *
 * Mirrored from Frontend Handoff §03 and no further: username is 3–50 of
 * [A-Za-z0-9_], full name is non-empty and at most 100.
 */

// ── username ───────────────────────────────────────────────────────────────

test('accepts letters, numbers and underscores', () => {
	expect(usernameError('jane_doe99')).toBeUndefined();
});

test('rejects anything else, in the spec’s words', () => {
	expect(usernameError('jane doe')).toBe(USERNAME_INVALID);
	expect(usernameError('jane-doe')).toBe(USERNAME_INVALID);
	expect(usernameError('jane.doe')).toBe(USERNAME_INVALID);
	expect(usernameError('jane@doe')).toBe(USERNAME_INVALID);
});

test('three characters is the floor', () => {
	expect(usernameError('jan')).toBeUndefined();
	expect(usernameError('ja')).toBe(USERNAME_TOO_SHORT);
	expect(usernameError('')).toBe(USERNAME_TOO_SHORT);
});

test('fifty is the ceiling, and it is a cap rather than a scolding', () => {
	expect(usernameError('a'.repeat(50))).toBeUndefined();
});

test('case is not the person’s mistake — the server lowercases it', () => {
	expect(usernameError('JaneDoe')).toBeUndefined();
});

test('surrounding whitespace is trimmed before judging', () => {
	expect(usernameError('  janedoe  ')).toBeUndefined();
});

// ── the public address it becomes ──────────────────────────────────────────

test('shows what the username will actually become', () => {
	// Permanent, and lowercased server-side. Someone typing JaneDoe should see
	// the address they are really getting before they commit to it, rather than
	// discovering it afterwards.
	expect(publicAddress('JaneDoe')).toBe('/janedoe');
	expect(publicAddress('  Jane_Doe  ')).toBe('/jane_doe');
});

test('says nothing until there is something to say', () => {
	expect(publicAddress('')).toBeUndefined();
	expect(publicAddress('   ')).toBeUndefined();
});

// ── full name ──────────────────────────────────────────────────────────────

test('a name is required, in the spec’s words', () => {
	expect(fullNameError('')).toBe(FULL_NAME_REQUIRED);
	expect(fullNameError('   ')).toBe(FULL_NAME_REQUIRED);
});

test('any non-empty name will do — it is not a format', () => {
	expect(fullNameError('Jane Doe')).toBeUndefined();
	expect(fullNameError('Ursula K. Le Guin')).toBeUndefined();
	expect(fullNameError('陳大文')).toBeUndefined();
});
