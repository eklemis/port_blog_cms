import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { POST as backendPOST } from '$lib/shared/api/client';
import type { components } from '$lib/shared/api/v1';
import { setAuthCookies } from '$lib/shared/auth/cookies.server';

/**
 * `POST /api/auth/login` — the only place a JWT is ever handled.
 *
 * The browser posts credentials here, this calls the backend, and both tokens
 * go into httpOnly cookies. What comes back is the user and nothing else: no
 * access token, no refresh token, nothing script can read.
 *
 * Two things this route deliberately does NOT do:
 *
 *   · It does not refuse an unverified account. Login succeeds for one and the
 *     tokens it issues are real — every authoring route is what says no, with
 *     403 EMAIL_NOT_VERIFIED. `user.is_verified` comes back so the client can
 *     route to the hold screen. Console Blueprint §02.
 *   · It does not translate a failure into prose. The error CODE is forwarded
 *     with the upstream status, because the code is the stable contract and the
 *     sentence a person reads is chosen by the copy table, not by the server.
 */

type LoginRequestDto = components['schemas']['LoginRequestDto'];
type ErrorDetail = components['schemas']['ErrorDetail'];
type ErrorResponse = components['schemas']['ErrorResponse'];

type LoginSuccess = {
	data: components['schemas']['LoginResponse'];
	success: true;
};

const UNSHAPED: ErrorDetail = {
	code: 'INTERNAL_ERROR',
	message: 'An unexpected error occurred'
};

/** An error body openapi-fetch could not parse is still an error we must name. */
function detailOf(error: unknown): ErrorDetail {
	const shaped = (error as ErrorResponse | undefined)?.error;
	return shaped?.code ? shaped : UNSHAPED;
}

export const POST: RequestHandler = async ({ request, cookies }) => {
	const body = (await request.json()) as LoginRequestDto;

	// The backend is unreachable when this is null. Answer in the shape the
	// client parses, so it reaches for a sentence rather than an unhandled 500.
	const result = await backendPOST('/api/auth/login', { body }).catch(() => null);

	if (!result) return json({ error: UNSHAPED }, { status: 502 });

	if (result.error) {
		const headers = new Headers();

		// Rate limits are a designed experience: the screen counts down against
		// the real number, so it has to survive the proxy. Blueprint §07.
		const retryAfter = result.response.headers.get('retry-after');
		if (retryAfter) headers.set('retry-after', retryAfter);

		return json({ error: detailOf(result.error) }, { status: result.response.status, headers });
	}

	const payload = (result.data as LoginSuccess).data;

	setAuthCookies(cookies, {
		access_token: payload.access_token,
		refresh_token: payload.refresh_token
	});

	return json({ user: payload.user });
};
