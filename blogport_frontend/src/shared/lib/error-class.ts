/**
 * Console Blueprint §07 — classify first, then write the sentence.
 *
 * The backend has 41 error codes across eight groups. People do not need 41
 * experiences; they need six, chosen by what they can do next. Getting the
 * class wrong produces a well-written sentence offering the wrong recovery,
 * which is worse than no sentence at all.
 *
 * The codes below are §07's table verbatim. When the backend adds one, it lands
 * in "not ours" until someone puts it in a row on purpose.
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
	| 'notOurs';

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
		'VALIDATION_ERROR'
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
	notOurs: ['STORAGE_ERROR', 'INTERNAL_ERROR', 'MEDIA_FAILED']
};

/** Built once. The lookup is per rendered alert, and the table does not change. */
const BY_CODE = new Map<string, HandlingClass>(
	Object.entries(CLASSES).flatMap(([name, codes]) =>
		codes.map((code) => [code, name as HandlingClass] as const)
	)
);

/**
 * Which of the six a code belongs to.
 *
 * Case-exact, because the wire is: wire enums are SCREAMING_SNAKE, and a
 * lenient match here would quietly accept a shape the API never sends and hide
 * a real drift. A code we do not know is "not ours" — it offers a retry and an
 * assurance rather than telling someone they got something wrong when a new
 * backend code is the only thing that changed.
 */
export function handlingClass(code: string | null | undefined): HandlingClass {
	if (!code) return 'notOurs';
	return BY_CODE.get(code) ?? 'notOurs';
}

export type AlertTone = 'neutral' | 'inflight' | 'danger';

/**
 * The colour each class carries.
 *
 * Red is spent only where something actually failed. A collision is a fork, a
 * gate is a step still owed and a wait is in progress — amber says "your move"
 * without claiming any of them went wrong. A session about to refresh has
 * nothing to report about the person's work at all.
 */
export const ALERT_TONE: Record<HandlingClass, AlertTone> = {
	field: 'danger',
	collision: 'inflight',
	session: 'neutral',
	gate: 'inflight',
	wait: 'inflight',
	notOurs: 'danger'
};
