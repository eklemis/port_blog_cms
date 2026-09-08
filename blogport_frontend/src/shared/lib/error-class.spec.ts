import { expect, test } from 'vitest';
import { handlingClass, ALERT_TONE } from './error-class';

/**
 * Console Blueprint §07: the backend has 41 error codes across eight groups,
 * and people need six experiences, chosen by what they can do next.
 *
 * The point of classifying first is that the sentence comes second. A code that
 * lands in the wrong class gets a well-written sentence offering the wrong
 * recovery, which is worse than no sentence at all.
 */

test('a rejected value is fixed and resubmitted', () => {
	for (const code of ['INVALID_EMAIL', 'INVALID_PASSWORD', 'EMPTY_TITLE', 'VALIDATION_ERROR']) {
		expect(handlingClass(code), code).toBe('field');
	}
});

test('a name already taken is a fork, not a failure', () => {
	// It gets a suggestion and a way to the thing that already exists.
	for (const code of ['USER_ALREADY_EXISTS', 'SLUG_ALREADY_EXISTS', 'TOPIC_ALREADY_EXISTS']) {
		expect(handlingClass(code), code).toBe('collision');
	}
});

test('anything about the token is the session, not the person', () => {
	for (const code of ['MISSING_AUTH_HEADER', 'TOKEN_EXPIRED', 'USER_UNAUTHORIZED']) {
		expect(handlingClass(code), code).toBe('session');
	}
});

test('the gate is its own class, and EMAIL_NOT_VERIFIED is the one that matters', () => {
	// It is why login is not the gate: sign-in succeeds and every authoring
	// route then 403s with this.
	expect(handlingClass('EMAIL_NOT_VERIFIED')).toBe('gate');
	expect(handlingClass('POST_UNAUTHORIZED')).toBe('gate');
	expect(handlingClass('USER_DELETED')).toBe('gate');
});

test('being told to come back later is waiting, not failing', () => {
	for (const code of ['MEDIA_PENDING', 'MEDIA_PROCESSING', 'RATE_LIMITED']) {
		expect(handlingClass(code), code).toBe('wait');
	}
});

test('their side breaking is scoped to the region that broke', () => {
	for (const code of ['STORAGE_ERROR', 'INTERNAL_ERROR', 'MEDIA_FAILED']) {
		expect(handlingClass(code), code).toBe('notOurs');
	}
});

test('a code we have never seen offers retry rather than blame', () => {
	// A new code on the backend must not make a working action look like the
	// person's mistake. "Not ours" is the safe wrong answer.
	expect(handlingClass('SOMETHING_NEW')).toBe('notOurs');
	expect(handlingClass(undefined)).toBe('notOurs');
	expect(handlingClass(null)).toBe('notOurs');
});

test('the mapping is case-exact, because the wire is', () => {
	// Wire enums are SCREAMING_SNAKE. A lowercase match here would quietly
	// accept a shape the API never sends and hide a real drift.
	expect(handlingClass('invalid_email')).toBe('notOurs');
});

// ── colour ─────────────────────────────────────────────────────────────────

test('every class has a tone, and only the ones that failed are red', () => {
	expect(Object.keys(ALERT_TONE).sort()).toEqual(
		['collision', 'field', 'gate', 'notOurs', 'session', 'wait'].sort()
	);

	expect(ALERT_TONE.field).toBe('danger');
	expect(ALERT_TONE.notOurs).toBe('danger');
	// A fork and a wait are not failures; amber says "your move" without
	// spending red on something that has gone nowhere wrong.
	expect(ALERT_TONE.collision).toBe('inflight');
	expect(ALERT_TONE.gate).toBe('inflight');
	expect(ALERT_TONE.wait).toBe('inflight');
	expect(ALERT_TONE.session).toBe('neutral');
});
