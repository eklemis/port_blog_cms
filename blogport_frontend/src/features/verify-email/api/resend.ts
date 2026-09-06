import { UNEXPECTED, rateLimited, retryAfterSeconds } from '$lib/shared/lib/api-failure';

/**
 * Ask for the verification link to be sent again.
 *
 * No arguments: the address lives in the session and the proxy reads it there,
 * so there is nothing for the browser to pass and nothing for it to get wrong.
 */

export const RESEND_ROUTE = '/api/auth/email-verification/resend';

/** J2's sentence. The hold screen is a waiting screen; sessions do end on it. */
export const SESSION_EXPIRED = 'Your session expired. Sign in to pick up where you left off.';

/**
 * The endpoint's own words when it has nothing to add. Deliberately
 * non-committal — the same text comes back whether or not the address needed
 * anything doing — and used verbatim because the Console Blueprint's copy table
 * has no entry for this success.
 */
const ACCEPTED = 'If that address needs verifying, a new link is on its way.';

export type ResendResult =
	| { ok: true; message: string }
	| { ok: false; message: string; retryAfterSeconds: number | null; signedOut: boolean };

function failed(
	message: string,
	{ seconds = null, signedOut = false }: { seconds?: number | null; signedOut?: boolean } = {}
): ResendResult {
	return { ok: false, message, retryAfterSeconds: seconds, signedOut };
}

export async function resendVerification(
	fetchFn: typeof globalThis.fetch = (...args) => globalThis.fetch(...args)
): Promise<ResendResult> {
	let response: Response;

	try {
		response = await fetchFn(RESEND_ROUTE, { method: 'POST' });
	} catch {
		return failed(UNEXPECTED);
	}

	const body = (await response.json().catch(() => null)) as {
		message?: string;
		error?: { code?: string };
	} | null;

	if (response.ok) return { ok: true, message: body?.message ?? ACCEPTED };

	switch (body?.error?.code) {
		case 'RATE_LIMITED': {
			const seconds = retryAfterSeconds(response);
			return failed(rateLimited(seconds), { seconds });
		}
		case 'MISSING_AUTH_HEADER':
		case 'INVALID_TOKEN':
		case 'TOKEN_EXPIRED':
			// Not a failure to report in place: there is nothing to press again
			// until they are signed in, so the caller sends them to do that.
			return failed(SESSION_EXPIRED, { signedOut: true });
		default:
			return failed(UNEXPECTED);
	}
}
