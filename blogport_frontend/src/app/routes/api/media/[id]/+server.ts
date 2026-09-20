import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { authenticatedFetch } from '$lib/shared/api/backend.server';
import type { components } from '$lib/shared/api/v1';

/**
 * `GET` and `DELETE /api/media/{id}` — poll one upload, or take it off.
 *
 * `GET` is the poll: "a row is created before the bytes arrive, so a media item
 * existing does not mean the file does." The card asks until `status` reaches
 * `ready`, because variants are produced by a separate service and "uploaded"
 * and "usable" are different moments.
 *
 * `DELETE` is a soft delete on the backend — the row drops out of every read
 * path at once, and the stored object is the bucket's lifecycle rule to reap.
 *
 * `PATCH` corrects the attachment's metadata. It is the endpoint that made the
 * frame's old "alt text cannot be edited later" false, and the reason the card
 * offers an Edit: "a missing or wrong alt text was a permanent accessibility
 * defect".
 */

type ErrorDetail = components['schemas']['ErrorDetail'];

const UNSHAPED: ErrorDetail = { code: 'INTERNAL_ERROR', message: 'An unexpected error occurred' };

function detailOf(body: unknown): ErrorDetail {
	const shaped = (body as { error?: ErrorDetail } | undefined)?.error;
	return shaped?.code ? shaped : UNSHAPED;
}

async function relay(
	event: Parameters<RequestHandler>[0],
	method: 'GET' | 'DELETE' | 'PATCH'
): Promise<Response> {
	const path = `/api/media/${encodeURIComponent(event.params.id ?? '')}`;

	// Passed through as text: the body is the caller's, and re-serialising it
	// here would be this route deciding what a correction may contain.
	const body = method === 'PATCH' ? await event.request.text() : undefined;

	let response: Response;

	try {
		response = await authenticatedFetch(event, path, {
			method,
			...(body === undefined ? {} : { headers: { 'content-type': 'application/json' }, body })
		});
	} catch {
		return json({ error: UNSHAPED }, { status: 502 });
	}

	const payload = await response.json().catch(() => null);

	if (!response.ok) {
		return json({ error: detailOf(payload) }, { status: response.status });
	}

	return json({ data: (payload as { data?: unknown })?.data ?? null });
}

export const GET: RequestHandler = (event) => relay(event, 'GET');

export const DELETE: RequestHandler = (event) => relay(event, 'DELETE');

export const PATCH: RequestHandler = (event) => relay(event, 'PATCH');
