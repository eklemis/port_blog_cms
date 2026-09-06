import { expect, test } from 'vitest';
import { CONSOLE_ROUTE, HOLD_ROUTE, destinationAfterSignIn, safeDestination } from './gate';

/**
 * Login is not the gate; verification is.
 *
 * POST /api/auth/login succeeds for an unverified account and hands back a
 * working access token, and every authoring route then refuses it with 403
 * EMAIL_NOT_VERIFIED. A console that reads "logged in" as "can work" shows a
 * person an authoring UI that fails on every button — so the decision about
 * where a successful sign-in lands is made here, from `is_verified`, and it is
 * made before anything renders. Console Blueprint §02, J1.
 */

const verified = { is_verified: true };
const unverified = { is_verified: false };

test('a verified author lands on the console', () => {
	expect(destinationAfterSignIn(verified, null)).toBe(CONSOLE_ROUTE);
	expect(CONSOLE_ROUTE).toBe('/studio');
});

test('a verified author returns to the destination they were sent away from', () => {
	expect(destinationAfterSignIn(verified, '/studio/posts/new')).toBe('/studio/posts/new');
});

test('an unverified account goes to the hold screen, never the console', () => {
	expect(destinationAfterSignIn(unverified, null)).toBe(HOLD_ROUTE);
	expect(HOLD_ROUTE).toBe('/verify');
});

test('a saved destination does not smuggle an unverified account past the gate', () => {
	// This is the failure the gate exists to prevent: a session-expiry redirect
	// carries ?next=/studio/posts/1, the account turns out to be unverified, and
	// honouring `next` would drop them straight into an editor that 403s.
	expect(destinationAfterSignIn(unverified, '/studio/posts/1')).toBe(HOLD_ROUTE);
});

// ── the saved destination is attacker-controlled input ─────────────────────

test('a same-site path is kept', () => {
	expect(safeDestination('/studio/media?page=2')).toBe('/studio/media?page=2');
});

test('nothing is kept when nothing was saved', () => {
	expect(safeDestination(null)).toBeNull();
	expect(safeDestination('')).toBeNull();
	expect(safeDestination('   ')).toBeNull();
});

test.each([
	['//evil.example/studio', 'a protocol-relative URL'],
	['https://evil.example/studio', 'an absolute URL'],
	['http://evil.example', 'an absolute URL on another origin'],
	['/\\evil.example', 'a backslash the browser reads as a slash'],
	['javascript:alert(1)', 'a script URL'],
	['studio', 'a relative path with no leading slash']
])('%s is discarded — %s', (candidate) => {
	expect(safeDestination(candidate)).toBeNull();
});

test('a discarded destination falls back to the console rather than failing', () => {
	expect(destinationAfterSignIn(verified, 'https://evil.example')).toBe(CONSOLE_ROUTE);
});
