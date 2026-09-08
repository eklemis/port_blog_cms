import type { components } from '$lib/shared/api/v1';
import { UNEXPECTED, rateLimited, retryAfterSeconds } from '$lib/shared/lib/api-failure';
import { normaliseEmail } from '$lib/shared/lib/email';

/**
 * Sign in through the SvelteKit proxy at `/api/auth/login`.
 *
 * The browser never holds a JWT: the proxy calls the backend, puts both tokens
 * in httpOnly cookies, and hands back the user. What is returned from here is a
 * user or a sentence — never a raw error code, which goes to the console and to
 * telemetry instead (Console Blueprint §06).
 */

type LoginUserInfo = components['schemas']['LoginUserInfo'];

export const LOGIN_ROUTE = '/api/auth/login';

/**
 * One message for a wrong address and a wrong password, deliberately. The
 * backend answers `INVALID_CREDENTIALS` for both so the endpoint cannot be used
 * to discover which addresses have accounts; distinguishing them here would
 * hand that back. Accessibility Spec §11 lists it as the one failure with no
 * suggested correction, for the same reason.
 */
export const SIGN_IN_FAILED = "That email and password don't match.";

/**
 * Not in the Console Blueprint's copy table — see the note in the PR. J2 wants a
 * closed account routed to a page of its own rather than answered on the sign-in
 * screen, and that page is not in the surface map yet.
 */
export const ACCOUNT_CLOSED = 'That account has been closed.';

export type SignInResult =
	| { ok: true; user: LoginUserInfo }
	| { ok: false; message: string; retryAfterSeconds: number | null };

function failed(message: string, seconds: number | null = null): SignInResult {
	return { ok: false, message, retryAfterSeconds: seconds };
}

export async function signIn(
	credentials: { email: string; password: string },
	fetchFn: typeof globalThis.fetch = (...args) => globalThis.fetch(...args)
): Promise<SignInResult> {
	let response: Response;

	try {
		response = await fetchFn(LOGIN_ROUTE, {
			method: 'POST',
			headers: { 'content-type': 'application/json' },
			body: JSON.stringify({
				email: normaliseEmail(credentials.email),
				// Never trimmed: normalisation for a password is "none".
				password: credentials.password
			})
		});
	} catch {
		// A dead network, a blocked request. Nothing the person did.
		return failed(UNEXPECTED);
	}

	const body = (await response.json().catch(() => null)) as {
		user?: LoginUserInfo;
		error?: { code?: string };
	} | null;

	if (response.ok && body?.user) return { ok: true, user: body.user };

	switch (body?.error?.code) {
		case 'INVALID_CREDENTIALS':
			return failed(SIGN_IN_FAILED);
		case 'USER_DELETED':
			return failed(ACCOUNT_CLOSED);
		case 'RATE_LIMITED': {
			const seconds = retryAfterSeconds(response);
			return failed(rateLimited(seconds), seconds);
		}
		default:
			// Anything else — including a code added to the API after this was
			// written — is ours to own rather than theirs to decipher.
			return failed(UNEXPECTED);
	}
}
