/**
 * End the session.
 *
 * The proxy sends the refresh token so the backend can blacklist it — that is
 * what stops a stolen refresh token outliving the session — and clears both
 * cookies. An access token stays cryptographically valid for up to 30 minutes
 * afterwards, so what actually ends the session in the browser is the cookies
 * going, which the proxy has already done by the time this resolves.
 */

export const LOGOUT_ROUTE = '/api/auth/logout';

/**
 * Never rejects. J2: clear local state regardless of the response — a failed
 * logout must never trap someone in a session they asked to end. The backend
 * confirms there is nothing to report anyway: a missing or unverifiable token
 * is logged and still answers 200.
 */
export async function signOut(
	fetchFn: typeof globalThis.fetch = (...args) => globalThis.fetch(...args)
): Promise<void> {
	await fetchFn(LOGOUT_ROUTE, { method: 'POST' }).catch(() => undefined);
}
