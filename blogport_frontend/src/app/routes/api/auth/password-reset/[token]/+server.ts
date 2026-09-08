import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { POST as backendPOST } from '$lib/shared/api/client';
import type { components } from '$lib/shared/api/v1';

/**
 * `POST /api/auth/password-reset/{token}` — consume the emailed token and set
 * the new password.
 *
 * The reset returns no tokens, so nobody is signed in afterwards: the backend
 * revokes every session belonging to the account, which is the point. The token
 * is never logged and never leaves the path.
 */

type ErrorDetail = components['schemas']['ErrorDetail'];

const UNSHAPED: ErrorDetail = { code: 'INTERNAL_ERROR', message: 'An unexpected error occurred' };

function detailOf(error: unknown): ErrorDetail {
	const shaped = (error as { error?: ErrorDetail } | undefined)?.error;
	return shaped?.code ? shaped : UNSHAPED;
}

export const POST: RequestHandler = async ({ request, params }) => {
	const body = (await request.json().catch(() => null)) as { password?: unknown } | null;
	const password = typeof body?.password === 'string' ? body.password : '';

	if (!password) {
		return json(
			{ error: { code: 'MISSING_FIELD', message: 'A password is required' } },
			{ status: 400 }
		);
	}

	const result = await backendPOST('/api/auth/password-reset/{token}', {
		params: { path: { token: params.token } },
		body: { password }
	}).catch(() => null);

	if (!result) return json({ error: UNSHAPED }, { status: 502 });

	if (result.error) {
		const headers = new Headers();
		// Ten an hour.
		const retryAfter = result.response.headers.get('retry-after');
		if (retryAfter) headers.set('retry-after', retryAfter);

		return json({ error: detailOf(result.error) }, { status: result.response.status, headers });
	}

	return json({ ok: true });
};
