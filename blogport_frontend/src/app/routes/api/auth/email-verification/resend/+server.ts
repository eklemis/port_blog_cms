import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { POST as backendPOST } from '$lib/shared/api/client';
import type { components } from '$lib/shared/api/v1';

/**
 * `POST /api/auth/email-verification/resend` — the one action the hold screen
 * offers.
 *
 * The address comes from the session and is never read from the request body.
 * The backend endpoint is public and answers `202` with the same words whether
 * the address is unknown, deleted, already verified or genuinely needed a link,
 * so that it cannot be used to discover which addresses are registered. That
 * protects the backend; it does not stop this origin from being used to send
 * mail to any address a caller names. Taking the address from the session
 * closes that, and costs nothing here — the screen is showing the signed-in
 * user their own address.
 */

type ErrorDetail = components['schemas']['ErrorDetail'];

type ResendSuccess = {
	data: components['schemas']['ResendVerificationResponse'];
	success: true;
};

const UNSHAPED: ErrorDetail = {
	code: 'INTERNAL_ERROR',
	message: 'An unexpected error occurred'
};

function detailOf(error: unknown): ErrorDetail {
	const shaped = (error as { error?: ErrorDetail } | undefined)?.error;
	return shaped?.code ? shaped : UNSHAPED;
}

export const POST: RequestHandler = async ({ locals }) => {
	const email = locals.user?.email;

	if (!email) {
		return json(
			{ error: { code: 'MISSING_AUTH_HEADER', message: 'Not signed in' } },
			{ status: 401 }
		);
	}

	const result = await backendPOST('/api/auth/email-verification/resend', {
		body: { email }
	}).catch(() => null);

	if (!result) return json({ error: UNSHAPED }, { status: 502 });

	if (result.error) {
		const headers = new Headers();

		// Five per hour, and each call mints a token and sends mail — this is a
		// limit someone waiting on an email will actually meet, so the countdown
		// has to be the real one.
		const retryAfter = result.response.headers.get('retry-after');
		if (retryAfter) headers.set('retry-after', retryAfter);

		return json({ error: detailOf(result.error) }, { status: result.response.status, headers });
	}

	// 202, not 200, because that is what it means: the request was accepted and
	// nothing about what followed is being reported.
	return json({ message: (result.data as ResendSuccess).data.message }, { status: 202 });
};
