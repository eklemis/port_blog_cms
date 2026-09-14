/**
 * Console Blueprint §07 — classify first, then write the sentence.
 *
 * Sixty codes, ten classes. People do not need sixty experiences; they need
 * ten, chosen by what they can do next. Getting the class wrong produces a
 * well-written sentence offering the wrong recovery, which is worse than no
 * sentence at all.
 *
 * The rule that shapes this module is §07's own: **the class is chosen by where
 * the code arrives, not by the code.** `TOKEN_EXPIRED` on an API call is a
 * session that died — refresh, then re-authenticate. The same code on a token
 * screen is an emailed link gone stale, and refreshing a session does nothing
 * for it. One string, two answers, so the string cannot answer alone.
 *
 * The rows below are §07's table verbatim, and the count is asserted against
 * `error_code.rs` in the spec: the last time this went stale it took
 * twenty-eight codes with it.
 */

export type HandlingClass =
	/** Under the input, on submit or on blur. Fix and resubmit; form state kept. */
	| 'field'
	/** Under the offending field, with a suggestion, or a way to the existing item. */
	| 'collision'
	/** Silent refresh, then a full-page redirect back to the saved destination. */
	| 'session'
	/** Route to /verify, or a full page for ownership. */
	| 'gate'
	/** In place, on the affected tile or button. Poll or retry; keep the placeholder. */
	| 'wait'
	/** Inline banner scoped to the failed region. Retry, and say other work is safe. */
	| 'notOurs'
	/** A full-page state where the item would have been, and a way back to its list. */
	| 'notFound'
	/** On the drop zone, before anything uploads. The form is untouched. */
	| 'fileRejected'
	/** In the companion rail. The document is never blocked on the model. */
	| 'generation'
	/** The token screen itself — never a redirect. A fresh link is sent from there. */
	| 'expiredLink';

/**
 * Where the answer came back. Most callers are API calls; a token screen is the
 * one place that knows it is something else, and it is the one that has to say.
 */
export type Arrival = 'api' | 'token-screen';

const CLASSES: Record<HandlingClass, readonly string[]> = {
	field: [
		'INVALID_EMAIL',
		'INVALID_PASSWORD',
		'INVALID_USERNAME',
		'INVALID_FULL_NAME',
		'INVALID_SLUG',
		'INVALID_TITLE',
		'EMPTY_TITLE',
		'TITLE_TOO_LONG',
		'INVALID_CONTENT',
		'MISSING_FIELD',
		'VALIDATION_ERROR',
		'INVALID_REQUEST',
		'INVALID_CREDENTIALS',
		// No field to land under, so they land on the control that submitted:
		// the selection toolbar, or the CV cell on a tracker row.
		'BULK_EMPTY',
		'BULK_TOO_LARGE',
		'SNAPSHOT_REQUIRED'
	],
	collision: ['USER_ALREADY_EXISTS', 'SLUG_ALREADY_EXISTS', 'TOPIC_ALREADY_EXISTS'],
	session: [
		'MISSING_AUTH_HEADER',
		'INVALID_TOKEN',
		'TOKEN_INVALID',
		'TOKEN_EXPIRED',
		'TOKEN_NOT_YET_VALID',
		'INVALID_TOKEN_TYPE',
		'USER_UNAUTHORIZED'
	],
	gate: ['EMAIL_NOT_VERIFIED', 'FORBIDDEN', 'CV_UNAUTHORIZED', 'POST_UNAUTHORIZED', 'USER_DELETED'],
	wait: ['MEDIA_PENDING', 'MEDIA_PROCESSING', 'RATE_LIMITED'],
	notOurs: ['STORAGE_ERROR', 'INTERNAL_ERROR', 'MEDIA_FAILED'],
	notFound: [
		'POST_NOT_FOUND',
		'PROJECT_NOT_FOUND',
		'CV_NOT_FOUND',
		'TOPIC_NOT_FOUND',
		'MEDIA_NOT_FOUND',
		'USER_NOT_FOUND',
		'JOB_NOT_FOUND',
		'APPLICATION_NOT_FOUND',
		'VARIANT_NOT_FOUND',
		'TARGET_NOT_FOUND'
	],
	fileRejected: [
		'FILE_TOO_LARGE',
		'INVALID_MIME_TYPE',
		'MIME_EXTENSION_MISMATCH',
		'INVALID_EXTENSION',
		'INVALID_FILE_NAME',
		'INVALID_DIMENSIONS'
	],
	generation: [
		'AI_DISABLED',
		'AI_QUOTA_EXCEEDED',
		'AI_REFUSED',
		'AI_TIMEOUT',
		'AI_UPSTREAM_ERROR',
		'AI_FETCH_FAILED'
	],
	// A reset token can only ever arrive on the screen the link opened, so it
	// needs no override below.
	expiredLink: ['INVALID_RESET_TOKEN']
};

/** Every code §07 classifies, flat. The spec counts it against the backend. */
export const CLASSIFIED: readonly string[] = Object.values(CLASSES).flat();

/** Built once. The lookup is per rendered alert; the table does not change. */
const BY_CODE = new Map<string, HandlingClass>(
	Object.entries(CLASSES).flatMap(([name, codes]) =>
		codes.map((code) => [code, name as HandlingClass] as const)
	)
);

/**
 * The two codes that mean something else on a token screen.
 *
 * Nothing else is reclassified by arriving there: a rate limit on the
 * reset-password screen is still a rate limit.
 */
const ON_TOKEN_SCREEN = new Set(['INVALID_TOKEN', 'TOKEN_EXPIRED']);

/**
 * Which of the ten a code belongs to, where it landed.
 *
 * Case-exact, because the wire is: wire enums are SCREAMING_SNAKE, and a
 * lenient match here would quietly accept a shape the API never sends and hide
 * a real drift. A code we do not know is "not ours" — it offers a retry and an
 * assurance rather than telling someone they got something wrong when a new
 * backend code is the only thing that changed.
 */
export function handlingClass(
	code: string | null | undefined,
	arrival: Arrival = 'api'
): HandlingClass {
	if (!code) return 'notOurs';
	if (arrival === 'token-screen' && ON_TOKEN_SCREEN.has(code)) return 'expiredLink';

	return BY_CODE.get(code) ?? 'notOurs';
}

export type AlertTone = 'neutral' | 'inflight' | 'danger';

/**
 * The colour each class carries.
 *
 * Red is spent only where something was refused — a value, a file, or the
 * request itself. A collision is a fork, a gate is a step still owed, a wait is
 * in progress, a stale link is replaceable and a model that did not answer has
 * broken nothing: amber says "your move" for all five. A session about to
 * refresh and a page for something that is not there both have nothing to
 * report about the person's work at all.
 */
export const ALERT_TONE: Record<HandlingClass, AlertTone> = {
	field: 'danger',
	collision: 'inflight',
	session: 'neutral',
	gate: 'inflight',
	wait: 'inflight',
	notOurs: 'danger',
	notFound: 'neutral',
	fileRejected: 'danger',
	generation: 'inflight',
	expiredLink: 'inflight'
};
