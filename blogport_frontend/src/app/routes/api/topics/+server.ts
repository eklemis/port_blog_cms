import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { authenticatedFetch } from '$lib/shared/api/backend.server';
import type { components } from '$lib/shared/api/v1';

/**
 * `POST /api/topics` — the editor picker's inline create.
 *
 * §03: "Create topic (inline) · title, description · Create & attach." Only
 * the create is here. The attach is a second request the editor makes with the
 * id this returns, because a topic that exists but is not yet on the post is a
 * truthful state and collapsing the two would hide which half failed.
 *
 * Reading topics needs no route: the editor's loader fetches them server-side
 * with the page.
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
		response = await authenticatedFetch(event, '/api/topics', {
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

	// Handed back whole: the editor needs the new topic's id to attach it.
	return json({ data: (payload as { data?: unknown })?.data ?? null }, { status: 201 });
};
