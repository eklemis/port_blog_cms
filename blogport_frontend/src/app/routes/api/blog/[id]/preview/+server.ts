import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { authenticatedFetch } from '$lib/shared/api/backend.server';
import type { components } from '$lib/shared/api/v1';

/**
 * `POST /api/blog/{id}/preview` — the share link behind the editor's Preview.
 *
 * It creates the link or extends the one that exists, so pressing Preview
 * twice is not two links to keep track of. §04: preview goes through the share
 * link rather than a private render, so the author checks exactly what a
 * reviewer will see.
 */

type ErrorDetail = components['schemas']['ErrorDetail'];

const UNSHAPED: ErrorDetail = { code: 'INTERNAL_ERROR', message: 'An unexpected error occurred' };

function detailOf(body: unknown): ErrorDetail {
	const shaped = (body as { error?: ErrorDetail } | undefined)?.error;
	return shaped?.code ? shaped : UNSHAPED;
}

export const POST: RequestHandler = async (event) => {
	const id = encodeURIComponent(event.params.id ?? '');

	let response: Response;

	try {
		response = await authenticatedFetch(event, `/api/blog/${id}/preview`, { method: 'POST' });
	} catch {
		return json({ error: UNSHAPED }, { status: 502 });
	}

	const payload = await response.json().catch(() => null);

	if (!response.ok) {
		return json({ error: detailOf(payload) }, { status: response.status });
	}

	return json((payload as { data?: unknown })?.data ?? null);
};
