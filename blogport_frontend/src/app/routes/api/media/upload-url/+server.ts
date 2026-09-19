import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { authenticatedFetch } from '$lib/shared/api/backend.server';
import type { components } from '$lib/shared/api/v1';

/**
 * `POST /api/media/upload-url` — begin a cover upload.
 *
 * The response carries a signed URL the browser PUTs the bytes to directly, so
 * this proxy passes a promise of somewhere else. It is the request that needs a
 * session, not the transfer.
 *
 * The 400 codes matter and are passed through whole: FILE_TOO_LARGE,
 * INVALID_MIME_TYPE, INVALID_DIMENSIONS and the rest are all `fileRejected` in
 * §07's table — "on the drop zone, before anything uploads" — and flattening
 * them to one sentence would leave the writer guessing which rule they broke.
 */

type ErrorDetail = components['schemas']['ErrorDetail'];

const UNSHAPED: ErrorDetail = { code: 'INTERNAL_ERROR', message: 'An unexpected error occurred' };

function detailOf(body: unknown): ErrorDetail {
	const shaped = (body as { error?: ErrorDetail } | undefined)?.error;
	return shaped?.code ? shaped : UNSHAPED;
}

export const POST: RequestHandler = async (event) => {
	const body = await event.request.text();

	let response: Response;

	try {
		response = await authenticatedFetch(event, '/api/media/upload-url', {
			method: 'POST',
			headers: { 'content-type': 'application/json' },
			body
		});
	} catch {
		return json({ error: UNSHAPED }, { status: 502 });
	}

	const payload = await response.json().catch(() => null);

	if (!response.ok) {
		return json({ error: detailOf(payload) }, { status: response.status });
	}

	// Both halves are needed: the id to poll, the URL to send to.
	return json({ data: (payload as { data?: unknown })?.data ?? null }, { status: 201 });
};
