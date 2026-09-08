import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { authenticatedFetch } from '$lib/shared/api/backend.server';
import type { components } from '$lib/shared/api/v1';

/**
 * `PATCH /api/blog/{id}` — the console's update proxy.
 *
 * Only the keys present in the body change, so the editor sends what moved and
 * never an object it did not load first. The error CODE is forwarded for the
 * same reason the create proxy forwards it: a slug collision is a branch.
 */

type PatchRequest = components['schemas']['PatchBlogPostRequest'];
type ErrorDetail = components['schemas']['ErrorDetail'];

const UNSHAPED: ErrorDetail = {
	code: 'INTERNAL_ERROR',
	message: 'An unexpected error occurred'
};

function detailOf(body: unknown): ErrorDetail {
	const shaped = (body as { error?: ErrorDetail } | undefined)?.error;
	return shaped?.code ? shaped : UNSHAPED;
}

export const PATCH: RequestHandler = async (event) => {
	const body = (await event.request.json()) as PatchRequest;
	const id = encodeURIComponent(event.params.id ?? '');

	let response: Response;

	try {
		response = await authenticatedFetch(event, `/api/blog/${id}`, {
			method: 'PATCH',
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

	return json((payload as { data?: unknown })?.data ?? null);
};
