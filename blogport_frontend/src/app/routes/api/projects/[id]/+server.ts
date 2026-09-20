import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { authenticatedFetch } from '$lib/shared/api/backend.server';
import type { components } from '$lib/shared/api/v1';

/**
 * `PATCH /api/projects/{id}` — field-level save.
 *
 * §02: "there is no full-replace PUT, so the editor is field-level throughout
 * and never sends an object it did not load first." The body is relayed whole
 * rather than rebuilt, so this route never decides what a save may contain.
 *
 * `SLUG_ALREADY_EXISTS` comes back as a 409 and is passed through with its
 * code: it belongs under the offending field, not in a banner.
 */

type ErrorDetail = components['schemas']['ErrorDetail'];

const UNSHAPED: ErrorDetail = { code: 'INTERNAL_ERROR', message: 'An unexpected error occurred' };

function detailOf(body: unknown): ErrorDetail {
	const shaped = (body as { error?: ErrorDetail } | undefined)?.error;
	return shaped?.code ? shaped : UNSHAPED;
}

export const PATCH: RequestHandler = async (event) => {
	const body = await event.request.text();

	let response: Response;

	try {
		response = await authenticatedFetch(
			event,
			`/api/projects/${encodeURIComponent(event.params.id ?? '')}`,
			{ method: 'PATCH', headers: { 'content-type': 'application/json' }, body }
		);
	} catch {
		return json({ error: UNSHAPED }, { status: 502 });
	}

	const payload = await response.json().catch(() => null);

	if (!response.ok) return json({ error: detailOf(payload) }, { status: response.status });

	return json({ data: (payload as { data?: unknown })?.data ?? null });
};
