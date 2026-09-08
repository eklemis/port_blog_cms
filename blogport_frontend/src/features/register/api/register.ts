import { UNEXPECTED, rateLimited, retryAfterSeconds } from '$lib/shared/lib/api-failure';
import { normaliseEmail } from '$lib/shared/lib/email';
import { normaliseUsername } from '../model/fields';

/**
 * Create the account.
 *
 * Registration returns a user and no token, so there is nothing to store and
 * nobody is signed in afterwards — J1 routes to the hold screen, and the proxy
 * leaves the address behind for it.
 */

export const REGISTER_ROUTE = '/api/auth/register';

/** Console Blueprint §07. The one branch with two ways out beside it. */
export const ADDRESS_TAKEN = "There's already an account for that address.";

/** Which field a failure belongs under, when it belongs under one at all. */
export type RegisterField = 'username' | 'email' | 'full_name' | 'password';

export type RegisterResult =
	| { ok: true }
	| {
			ok: false;
			field: RegisterField | null;
			message: string;
			/** The address is taken — the screen offers signing in and resetting. */
			collision: boolean;
			retryAfterSeconds: number | null;
	  };

const FIELD_OF: Record<string, RegisterField> = {
	INVALID_USERNAME: 'username',
	INVALID_EMAIL: 'email',
	INVALID_FULL_NAME: 'full_name',
	INVALID_PASSWORD: 'password'
};

function failed(
	message: string,
	{
		field = null,
		collision = false,
		seconds = null
	}: { field?: RegisterField | null; collision?: boolean; seconds?: number | null } = {}
): RegisterResult {
	return { ok: false, field, message, collision, retryAfterSeconds: seconds };
}

export async function register(
	details: { username: string; email: string; full_name: string; password: string },
	fetchFn: typeof globalThis.fetch = (...args) => globalThis.fetch(...args)
): Promise<RegisterResult> {
	let response: Response;

	try {
		response = await fetchFn(REGISTER_ROUTE, {
			method: 'POST',
			headers: { 'content-type': 'application/json' },
			body: JSON.stringify({
				username: normaliseUsername(details.username),
				email: normaliseEmail(details.email),
				full_name: details.full_name.trim(),
				// Never trimmed: normalisation for a password is "none".
				password: details.password
			})
		});
	} catch {
		return failed(UNEXPECTED);
	}

	const body = (await response.json().catch(() => null)) as {
		error?: { code?: string; message?: string };
	} | null;

	if (response.ok) return { ok: true };

	const code = body?.error?.code ?? '';

	if (code === 'USER_ALREADY_EXISTS') {
		return failed(ADDRESS_TAKEN, { field: 'email', collision: true });
	}

	if (code === 'RATE_LIMITED') {
		const seconds = retryAfterSeconds(response);
		// J1 names the action: someone who has hit a limit wants to know which.
		return failed(rateLimited(seconds, 'Too many sign-up attempts'), { seconds });
	}

	const field = FIELD_OF[code];
	if (field) {
		// The server's message names the failed rule. Ours mirrors the same rule,
		// so arriving here means they disagreed — and the server is the authority
		// on what it will accept.
		return failed(body?.error?.message ?? UNEXPECTED, { field });
	}

	return failed(UNEXPECTED);
}
