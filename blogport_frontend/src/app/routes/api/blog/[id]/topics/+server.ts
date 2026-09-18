import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { authenticatedFetch } from '$lib/shared/api/backend.server';
import type { components } from '$lib/shared/api/v1';

/**
 * `POST`/`DELETE /api/blog/{id}/topics` — one chip on, one chip off.
 *
 * The browser cannot call the backend itself: it holds no bearer token, and
 * the session lives in this app's cookies. Every write the editor makes goes
 * through a route like this one.
 *
 * §03: each chip is its own request, so these forward one topic and say what
 * happened to it — a refusal keeps the backend's own code, because the editor
 * classifies by it.
 */

type ErrorDetail = components['schemas']['ErrorDetail'];

const UNSHAPED: ErrorDetail = { code: 'INTERNAL_ERROR', message: 'An unexpected error occurred' };

function detailOf(body: unknown): ErrorDetail {
	const shaped = (body as { error?: ErrorDetail } | undefined)?.error;
	return shaped?.code ? shaped : UNSHAPED;
}

async function forward(event: Parameters<RequestHandler>[0], method: 'POST' | 'DELETE') {
	const id = encodeURIComponent(event.params.id ?? '');
	const body = await event.request.text();

	let response: Response;

	try {
		response = await authenticatedFetch(event, `/api/blog/${id}/topics`, {
			method,
			headers: { 'content-type': 'application/json' },
			body
		});
	} catch {
		return json({ error: UNSHAPED }, { status: 502 });
	}

	if (!response.ok) {
		const payload = await response.json().catch(() => null);
		return json({ error: detailOf(payload) }, { status: response.status });
	}

	// Nothing to hand back: the editor already knows which chip it sent.
	return new Response(null, { status: 204 });
}

export const POST: RequestHandler = (event) => forward(event, 'POST');
export const DELETE: RequestHandler = (event) => forward(event, 'DELETE');
