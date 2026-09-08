import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { authenticatedFetch } from '$lib/shared/api/backend.server';
import type { components } from '$lib/shared/api/v1';

/**
 * `POST /api/blog` — the console's create proxy.
 *
 * A console mutation goes out through here for the same reason a console list
 * comes in through a loader: the browser never holds a JWT, and the access
 * token lives in an httpOnly cookie that only the server can read.
 *
 * Like the auth proxies, it forwards the error CODE and not only the prose. A
 * slug collision is a branch that offers a suggestion; a sentence cannot be
 * branched on.
 */

type CreateRequest = components['schemas']['CreateBlogPostRequest'];
type ErrorDetail = components['schemas']['ErrorDetail'];

const UNSHAPED: ErrorDetail = {
	code: 'INTERNAL_ERROR',
	message: 'An unexpected error occurred'
};

function detailOf(body: unknown): ErrorDetail {
	const shaped = (body as { error?: ErrorDetail } | undefined)?.error;
	return shaped?.code ? shaped : UNSHAPED;
}

export const POST: RequestHandler = async (event) => {
	const body = (await event.request.json()) as CreateRequest;

	let response: Response;

	try {
		response = await authenticatedFetch(event, '/api/blog', {
			method: 'POST',
			headers: { 'content-type': 'application/json' },
			body: JSON.stringify(body)
		});
	} catch {
		return json({ error: UNSHAPED }, { status: 502 });
	}

	const payload = await response.json().catch(() => null);

	if (!response.ok) {
		const headers = new Headers();

		const retryAfter = response.headers.get('retry-after');
		if (retryAfter) headers.set('retry-after', retryAfter);

		return json({ error: detailOf(payload) }, { status: response.status, headers });
	}

	return json((payload as { data?: unknown })?.data ?? null, { status: 201 });
};
