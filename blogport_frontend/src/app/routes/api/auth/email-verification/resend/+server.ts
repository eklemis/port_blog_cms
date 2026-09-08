import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { POST as backendPOST } from '$lib/shared/api/client';
import type { components } from '$lib/shared/api/v1';

/**
 * `POST /api/auth/email-verification/resend` — the one action the hold screen
 * offers.
 *
 * The address comes from the request body when one is given, and from the
 * session otherwise.
 *
 * This route used to require a session. That was a hardening decision of mine
 * rather than a constraint the API imposes — the operation carries no security
 * requirement and takes `{ email }` — and it broke the screen that needs it
 * most: an expired link opened from mail on a phone has no session, which is
 * exactly the moment a new link has to be asked for. The backend's own defences
 * are the right ones: it answers 202 identically whether the address is
 * unknown, deleted, already verified or genuinely waiting, so it cannot be used
 * to discover which addresses are registered, and it rate-limits 5 per hour per
 * caller because each call mints a token and sends mail.
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

export const POST: RequestHandler = async ({ locals, request }) => {
	const body = (await request.json().catch(() => null)) as { email?: unknown } | null;
	const supplied = typeof body?.email === 'string' ? body.email.trim() : '';

	// A supplied address wins: someone signed in who mistyped their address at
	// registration could not otherwise ask for a link to the right one.
	const email = supplied || (locals.user?.email ?? '');

	if (!email) {
		return json(
			{ error: { code: 'MISSING_FIELD', message: 'An email address is required' } },
			{ status: 400 }
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
