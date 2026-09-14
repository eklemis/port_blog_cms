import { expect, test } from 'vitest';
import { ALERT_TONE, CLASSIFIED, handlingClass } from './error-class';

/**
 * Console Blueprint §07 — classify first, then write the sentence.
 *
 * Sixty codes, ten classes. It was six classes and forty-one codes when this
 * was first written; twenty-eight codes had no class at all, which the designer
 * found by chasing a one-row question about a dead reset link.
 *
 * The rule that shapes this module: the class is chosen by where the code
 * arrives, not by the code. One string, two answers.
 */

test('a rejected value is fixed and resubmitted', () => {
	for (const code of ['INVALID_EMAIL', 'INVALID_PASSWORD', 'EMPTY_TITLE', 'VALIDATION_ERROR']) {
		expect(handlingClass(code), code).toBe('field');
	}
});

test('a bulk action with no field lands on the control that submitted it', () => {
	// Field-level without a field: the selection toolbar, or the CV cell on a
	// tracker row. Still the class, because the recovery is the same.
	for (const code of ['BULK_EMPTY', 'BULK_TOO_LARGE', 'SNAPSHOT_REQUIRED']) {
		expect(handlingClass(code), code).toBe('field');
	}
});

test('a name already taken is a fork, not a failure', () => {
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

// ── the four that were missing ─────────────────────────────────────────────

test('something that is not there is a page, not a message under a field', () => {
	for (const code of ['POST_NOT_FOUND', 'JOB_NOT_FOUND', 'VARIANT_NOT_FOUND']) {
		expect(handlingClass(code), code).toBe('notFound');
	}
});

test('a refused file is answered on the drop zone, before anything uploads', () => {
	for (const code of ['FILE_TOO_LARGE', 'INVALID_MIME_TYPE', 'INVALID_DIMENSIONS']) {
		expect(handlingClass(code), code).toBe('fileRejected');
	}
});

test('the model failing never blocks the document', () => {
	for (const code of ['AI_DISABLED', 'AI_QUOTA_EXCEEDED', 'AI_TIMEOUT']) {
		expect(handlingClass(code), code).toBe('generation');
	}
});

test('a dead emailed link is answered on the screen it landed on', () => {
	expect(handlingClass('INVALID_RESET_TOKEN')).toBe('expiredLink');
});

// ── one code, two classes ──────────────────────────────────────────────────

test('where it arrives decides, not the string', () => {
	// TOKEN_EXPIRED on an API call is a session that died: refresh, then
	// re-authenticate. The same code on a token screen is a stale link, and
	// refreshing a session does nothing for it.
	expect(handlingClass('TOKEN_EXPIRED', 'api')).toBe('session');
	expect(handlingClass('TOKEN_EXPIRED', 'token-screen')).toBe('expiredLink');

	expect(handlingClass('INVALID_TOKEN', 'api')).toBe('session');
	expect(handlingClass('INVALID_TOKEN', 'token-screen')).toBe('expiredLink');
});

test('an API call is what a caller gets for not saying', () => {
	// Most callers are API calls, and a token screen knows that it is one.
	expect(handlingClass('TOKEN_EXPIRED')).toBe('session');
});

test('a token screen does not reclassify codes that are not about tokens', () => {
	expect(handlingClass('RATE_LIMITED', 'token-screen')).toBe('wait');
	expect(handlingClass('INTERNAL_ERROR', 'token-screen')).toBe('notOurs');
});

// ── the table itself ───────────────────────────────────────────────────────

test('every code the API can send has a class', () => {
	// Sixty, counted from `error_code.rs`. The last time this table went stale
	// it took twenty-eight codes with it, so the count is asserted rather than
	// assumed.
	expect(CLASSIFIED).toHaveLength(60);
	expect(new Set(CLASSIFIED).size, 'a code in two rows').toBe(60);
});

test('a code we have never seen offers retry rather than blame', () => {
	expect(handlingClass('SOMETHING_NEW')).toBe('notOurs');
	expect(handlingClass(undefined)).toBe('notOurs');
	expect(handlingClass(null)).toBe('notOurs');
});

test('the mapping is case-exact, because the wire is', () => {
	expect(handlingClass('invalid_email')).toBe('notOurs');
});

// ── colour ─────────────────────────────────────────────────────────────────

test('every class has a tone, and only the ones that failed are red', () => {
	expect(Object.keys(ALERT_TONE)).toHaveLength(10);

	expect(ALERT_TONE.field).toBe('danger');
	expect(ALERT_TONE.notOurs).toBe('danger');
	// A refused file is a refusal: this one is on the person's side to fix.
	expect(ALERT_TONE.fileRejected).toBe('danger');

	// None of these failed. Amber says "your move" without spending red.
	expect(ALERT_TONE.collision).toBe('inflight');
	expect(ALERT_TONE.gate).toBe('inflight');
	expect(ALERT_TONE.wait).toBe('inflight');
	expect(ALERT_TONE.expiredLink).toBe('inflight');
	expect(ALERT_TONE.generation).toBe('inflight');

	expect(ALERT_TONE.session).toBe('neutral');
	// A page where the thing would have been, not a warning about it.
	expect(ALERT_TONE.notFound).toBe('neutral');
});
