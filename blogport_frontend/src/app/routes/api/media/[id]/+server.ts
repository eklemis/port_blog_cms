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
 */

type ErrorDetail = components['schemas']['ErrorDetail'];

const UNSHAPED: ErrorDetail = { code: 'INTERNAL_ERROR', message: 'An unexpected error occurred' };

function detailOf(body: unknown): ErrorDetail {
	const shaped = (body as { error?: ErrorDetail } | undefined)?.error;
	return shaped?.code ? shaped : UNSHAPED;
}

async function relay(
	event: Parameters<RequestHandler>[0],
	method: 'GET' | 'DELETE'
): Promise<Response> {
	const path = `/api/media/${encodeURIComponent(event.params.id ?? '')}`;

	let response: Response;

	try {
		response = await authenticatedFetch(event, path, { method });
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
