import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { authenticatedFetch } from '$lib/shared/api/backend.server';
import type { components } from '$lib/shared/api/v1';

/**
 * `POST` and `DELETE /api/projects/{id}/topics` — one chip at a time.
 *
 * §03: "each chip is its own request; one failure doesn't roll back the
 * others." A rail that half-succeeded is a rail telling the truth.
 */

type ErrorDetail = components['schemas']['ErrorDetail'];

const UNSHAPED: ErrorDetail = { code: 'INTERNAL_ERROR', message: 'An unexpected error occurred' };

function detailOf(body: unknown): ErrorDetail {
	const shaped = (body as { error?: ErrorDetail } | undefined)?.error;
	return shaped?.code ? shaped : UNSHAPED;
}

async function relay(
	event: Parameters<RequestHandler>[0],
	method: 'POST' | 'DELETE'
): Promise<Response> {
	const body = await event.request.text();

	let response: Response;

	try {
		response = await authenticatedFetch(
			event,
			`/api/projects/${encodeURIComponent(event.params.id ?? '')}/topics`,
			{ method, headers: { 'content-type': 'application/json' }, body }
		);
	} catch {
		return json({ error: UNSHAPED }, { status: 502 });
	}

	const payload = await response.json().catch(() => null);

	if (!response.ok) return json({ error: detailOf(payload) }, { status: response.status });

	return json({ data: (payload as { data?: unknown })?.data ?? null });
}

export const POST: RequestHandler = (event) => relay(event, 'POST');

export const DELETE: RequestHandler = (event) => relay(event, 'DELETE');
