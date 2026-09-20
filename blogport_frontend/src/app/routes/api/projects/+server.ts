import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { authenticatedFetch } from '$lib/shared/api/backend.server';
import type { components } from '$lib/shared/api/v1';

/**
 * `POST /api/projects` — and creating one publishes it.
 *
 * `SLUG_ALREADY_EXISTS` comes back as a 409 and is passed through with its
 * code: §02 asks for "suggest, don't discard", which the form can only do if it
 * knows which rule was broken.
 *
 * Reading the list needs no route here — the list's loader fetches it
 * server-side with the page.
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
		response = await authenticatedFetch(event, '/api/projects', {
			method: 'POST',
			headers: { 'content-type': 'application/json' },
			body
		});
	} catch {
		return json({ error: UNSHAPED }, { status: 502 });
	}

	const payload = await response.json().catch(() => null);

	if (!response.ok) return json({ error: detailOf(payload) }, { status: response.status });

	// The id is what the caller needs: it is where the editor lives.
	return json({ data: (payload as { data?: unknown })?.data ?? null }, { status: 201 });
};
