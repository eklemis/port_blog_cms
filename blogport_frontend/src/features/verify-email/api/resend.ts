import { UNEXPECTED, rateLimited, retryAfterSeconds } from '$lib/shared/lib/api-failure';
import { normaliseEmail } from '$lib/shared/lib/email';
import type { HandlingClass } from '$lib/shared/lib/error-class';

/**
 * Ask for the verification link to be sent again.
 *
 * The address is optional: the hold screen has a session and lets the proxy
 * read it there, while the expired-link screen has neither and asks the person
 * for it.
 */

export const RESEND_ROUTE = '/api/auth/email-verification/resend';

/** J2's sentence. The hold screen is a waiting screen; sessions do end on it. */
export const SESSION_EXPIRED = 'Your session expired. Sign in to pick up where you left off.';

/**
 * Ours, not the backend's sentence: an API message is written for a developer,
 * and this one is read by the person waiting. Console Blueprint §07.
 *
 * Two of them, because the two screens know different things. On the hold
 * screen the address is the session's own, so there is nothing to be coy about.
 * On the expired-link screen it was typed, and confirming that a particular
 * address was worth sending to would say whether it has an account — the same
 * enumeration rule J3 applies to password reset.
 */
export const RESEND_SENT = 'Sent. Check your inbox — the new link is good for 24 hours.';
export const RESEND_SENT_NEUTRAL =
	'If that address is waiting to be verified, a new link is on its way. It is good for 24 hours.';

export type ResendResult =
	| { ok: true }
	| {
			ok: false;
			message: string;
			retryAfterSeconds: number | null;
			signedOut: boolean;
			/** Which of §07's six this is, so the caller colours it without a code. */
			kind: HandlingClass;
	  };

/** `notOurs` by default, the same fallback the mapping itself takes. */
function failed(
	message: string,
	{
		seconds = null,
		signedOut = false,
		kind = 'notOurs'
	}: { seconds?: number | null; signedOut?: boolean; kind?: HandlingClass } = {}
): ResendResult {
	return { ok: false, message, retryAfterSeconds: seconds, signedOut, kind };
}

export async function resendVerification(
	email?: string,
	fetchFn: typeof globalThis.fetch = (...args) => globalThis.fetch(...args)
): Promise<ResendResult> {
	let response: Response;

	try {
		response = await fetchFn(RESEND_ROUTE, {
			method: 'POST',
			headers: { 'content-type': 'application/json' },
			// An empty body when there is no address: the proxy falls back to the
			// session, which is the hold screen's case.
			body: JSON.stringify(email ? { email: normaliseEmail(email) } : {})
		});
	} catch {
		return failed(UNEXPECTED);
	}

	const body = (await response.json().catch(() => null)) as { error?: { code?: string } } | null;

	if (response.ok) return { ok: true };

	switch (body?.error?.code) {
		case 'RATE_LIMITED': {
			const seconds = retryAfterSeconds(response);
			return failed(rateLimited(seconds), { seconds, kind: 'wait' });
		}
		case 'MISSING_AUTH_HEADER':
		case 'INVALID_TOKEN':
		case 'TOKEN_EXPIRED':
			// Not a failure to report in place: there is nothing to press again
			// until they are signed in, so the caller sends them to do that.
			return failed(SESSION_EXPIRED, { signedOut: true, kind: 'session' });
		default:
			return failed(UNEXPECTED);
	}
}
