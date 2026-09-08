import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { POST as backendPOST } from '$lib/shared/api/client';
import type { components } from '$lib/shared/api/v1';

/**
 * `POST /api/auth/password-reset` — ask for a reset link.
 *
 * Public and deliberately non-committal: the backend answers 200 with the same
 * words whether or not the address belongs to an account, because branching
 * would turn the form into a way of discovering who is registered. Nothing here
 * may narrow that, which is why no session is required and none is consulted.
 */

type ErrorDetail = components['schemas']['ErrorDetail'];

const UNSHAPED: ErrorDetail = { code: 'INTERNAL_ERROR', message: 'An unexpected error occurred' };

function detailOf(error: unknown): ErrorDetail {
	const shaped = (error as { error?: ErrorDetail } | undefined)?.error;
	return shaped?.code ? shaped : UNSHAPED;
}

export const POST: RequestHandler = async ({ request }) => {
	const body = (await request.json().catch(() => null)) as { email?: unknown } | null;
	const email = typeof body?.email === 'string' ? body.email.trim() : '';

	if (!email) {
		return json(
			{ error: { code: 'MISSING_FIELD', message: 'An email address is required' } },
			{ status: 400 }
		);
	}

	const result = await backendPOST('/api/auth/password-reset', { body: { email } }).catch(
		() => null
	);

	if (!result) return json({ error: UNSHAPED }, { status: 502 });

	if (result.error) {
		const headers = new Headers();
		// Five an hour. Each call mints a token and sends mail.
		const retryAfter = result.response.headers.get('retry-after');
		if (retryAfter) headers.set('retry-after', retryAfter);

		return json({ error: detailOf(result.error) }, { status: result.response.status, headers });
	}

	return json({ ok: true });
};
