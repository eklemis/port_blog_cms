import { UNEXPECTED, rateLimited, retryAfterSeconds } from '$lib/shared/lib/api-failure';
import { normaliseEmail } from '$lib/shared/lib/email';
import type { HandlingClass } from '$lib/shared/lib/error-class';

/**
 * Recovering a password, in two halves: ask for a link, then use it.
 *
 * The whole journey is built so that a stranger cannot learn whether an address
 * is registered — the request answers identically either way, and nothing on
 * this side branches on it.
 */

export const REQUEST_ROUTE = '/api/auth/password-reset';

/** Console Blueprint §07, and the branch J3 names. */
export const RESET_LINK_DEAD = 'This reset link is no longer valid.';

/**
 * The confirmation, named rather than found: it says "if", every time, whatever
 * the server did. Screen / Forgot password 67:136.
 */
export function resetRequested(email: string): string {
	return `If an account exists for ${normaliseEmail(email)}, a reset link is on its way. It is good for one hour.`;
}

export type ResetResult =
	| { ok: true }
	| {
			ok: false;
			message: string;
			retryAfterSeconds: number | null;
			expired: boolean;
			/** Which of §07's six this is, so the caller colours it without a code. */
			kind: HandlingClass;
	  };

/** `notOurs` by default, the same fallback the mapping itself takes. */
function failed(
	message: string,
	{
		seconds = null,
		expired = false,
		kind = 'notOurs'
	}: { seconds?: number | null; expired?: boolean; kind?: HandlingClass } = {}
): ResetResult {
	return { ok: false, message, retryAfterSeconds: seconds, expired, kind };
}

async function readError(response: Response) {
	const body = (await response.json().catch(() => null)) as {
		error?: { code?: string; message?: string };
	} | null;
	return body?.error;
}

/** Ask for a link. Never reports whether the address was found. */
export async function requestReset(
	email: string,
	fetchFn: typeof globalThis.fetch = (...args) => globalThis.fetch(...args)
): Promise<ResetResult> {
	let response: Response;

	try {
		response = await fetchFn(REQUEST_ROUTE, {
			method: 'POST',
			headers: { 'content-type': 'application/json' },
			body: JSON.stringify({ email: normaliseEmail(email) })
		});
	} catch {
		return failed(UNEXPECTED);
	}

	if (response.ok) return { ok: true };

	const error = await readError(response);

	if (error?.code === 'RATE_LIMITED') {
		const seconds = retryAfterSeconds(response);
		return failed(rateLimited(seconds), { seconds, kind: 'wait' });
	}

	return failed(UNEXPECTED);
}

/**
 * Use the link. On success every session belonging to the account is revoked
 * server-side, and no token comes back — nobody is signed in afterwards.
 */
export async function setPassword(
	token: string,
	password: string,
	fetchFn: typeof globalThis.fetch = (...args) => globalThis.fetch(...args)
): Promise<ResetResult> {
	let response: Response;

	try {
		response = await fetchFn(`${REQUEST_ROUTE}/${encodeURIComponent(token)}`, {
			method: 'POST',
			headers: { 'content-type': 'application/json' },
			// Never trimmed: normalisation for a password is "none".
			body: JSON.stringify({ password })
		});
	} catch {
		return failed(UNEXPECTED);
	}

	if (response.ok) return { ok: true };

	const error = await readError(response);

	switch (error?.code) {
		case 'RATE_LIMITED': {
			const seconds = retryAfterSeconds(response);
			return failed(rateLimited(seconds), { seconds, kind: 'wait' });
		}
		case 'INVALID_RESET_TOKEN':
		case 'TOKEN_EXPIRED':
		case 'INVALID_TOKEN':
		case 'USER_NOT_FOUND':
			// J3: offer a fresh one from this same screen rather than bouncing.
			// §07 files a dead token under session. That class is written for an
			// auth token, where the recovery is to sign in again; here it is a
			// fresh link. The distinction is flagged rather than settled locally,
			// and it changes nothing on screen — this branch drives its own.
			return failed(RESET_LINK_DEAD, { expired: true, kind: 'session' });
		case 'INVALID_PASSWORD':
			// Keep the token in the URL and let them retype — losing a valid token
			// to one weak password is a needless restart. The server's message
			// names the rule it failed.
			return failed(error.message ?? UNEXPECTED, { kind: 'field' });
		default:
			return failed(UNEXPECTED);
	}
}
