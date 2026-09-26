import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { authenticatedFetch } from '$lib/shared/api/backend.server';
import type { components } from '$lib/shared/api/v1';

/**
 * `PATCH /api/cvs/{id}` — field-level, and every collection on it replaces
 * wholesale: "There is no per-item patch: a list is replaced or left alone."
 *
 * The body is relayed as text so this route never decides what a résumé may
 * contain. `CV_UNAUTHORIZED` is passed through with its code — §07 files it
 * under gate, which is a different screen from a failure.
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
			`/api/cvs/${encodeURIComponent(event.params.id ?? '')}`,
			{
				method: 'PATCH',
				headers: { 'content-type': 'application/json' },
				body
			}
		);
	} catch {
		return json({ error: UNSHAPED }, { status: 502 });
	}

	const payload = await response.json().catch(() => null);

	if (!response.ok) return json({ error: detailOf(payload) }, { status: response.status });

	return json({ data: (payload as { data?: unknown })?.data ?? null });
};
