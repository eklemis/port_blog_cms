import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { POST as backendPOST } from '$lib/shared/api/client';
import type { components } from '$lib/shared/api/v1';
import { setPendingVerificationEmail } from '$lib/shared/auth/cookies.server';

/**
 * `POST /api/auth/register`.
 *
 * Registration returns a user and no token, so nothing here establishes a
 * session — J1 is explicit that the next screen is the hold screen and not a
 * dashboard. What this does leave behind is the address the link went to, in a
 * cookie, because /verify is next and has no other way to know it.
 *
 * Like the login proxy, it forwards the error CODE rather than only the prose:
 * 409 is the branch that offers "Sign in instead", and a sentence cannot be
 * branched on.
 */

type RegisterRequest = components['schemas']['CreateUserRequest'];
type RegisterSuccess = components['schemas']['SuccessResponse_RegisterUserResponse'];
type ErrorDetail = components['schemas']['ErrorDetail'];

const UNSHAPED: ErrorDetail = {
	code: 'INTERNAL_ERROR',
	message: 'An unexpected error occurred'
};

function detailOf(error: unknown): ErrorDetail {
	const shaped = (error as { error?: ErrorDetail } | undefined)?.error;
	return shaped?.code ? shaped : UNSHAPED;
}

export const POST: RequestHandler = async ({ request, cookies }) => {
	const body = (await request.json()) as RegisterRequest;

	const result = await backendPOST('/api/auth/register', { body }).catch(() => null);

	if (!result) return json({ error: UNSHAPED }, { status: 502 });

	if (result.error) {
		const headers = new Headers();

		// Five sign-ups an hour, and J1 wants a real countdown against the real
		// number rather than a guess.
		const retryAfter = result.response.headers.get('retry-after');
		if (retryAfter) headers.set('retry-after', retryAfter);

		return json({ error: detailOf(result.error) }, { status: result.response.status, headers });
	}

	const payload = (result.data as RegisterSuccess).data;

	setPendingVerificationEmail(cookies, body.email);

	return json(payload, { status: 201 });
};
