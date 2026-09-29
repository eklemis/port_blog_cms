import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { authenticatedFetch } from '$lib/shared/api/backend.server';
import type { components } from '$lib/shared/api/v1';

/**
 * `POST /api/cvs` — the list's "+ New résumé", J6's first step.
 *
 * The browser cannot call the API directly: the access token lives in an
 * httpOnly cookie, so a write started by a component goes through a route like
 * this one, which is where the cookie is.
 *
 * The body is relayed as text, so this route never decides what a résumé may
 * contain — `CreateCVRequest` is the API's rule and the API is what enforces
 * it. The refusal keeps its code for the same reason the PATCH beside this one
 * does: §07 picks the screen from the code, and a flattened 500 would offer the
 * wrong recovery.
 *
 * Reading the list needs no route — the page's loader fetches it server-side.
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
		response = await authenticatedFetch(event, '/api/cvs', {
			method: 'POST',
			headers: { 'content-type': 'application/json' },
			body
		});
	} catch {
		return json({ error: UNSHAPED }, { status: 502 });
	}

	const payload = await response.json().catch(() => null);

	if (!response.ok) return json({ error: detailOf(payload) }, { status: response.status });

	// Handed back whole: the list needs the new résumé's id to open its builder.
	return json({ data: (payload as { data?: unknown })?.data ?? null }, { status: 201 });
};
