import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { authenticatedFetch } from '$lib/shared/api/backend.server';
import type { components } from '$lib/shared/api/v1';

/**
 * `DELETE /api/blog/{id}/hard` — the third rung, and the only one with no way back.
 *
 * Reached only from the archive, and only through ConfirmDialog. This proxy trusts that the confirmation happened; it has no way to know.
 *
 * 204 on success, so there is no body to forward; the error CODE is forwarded
 * when there is one, for the same reason every console proxy forwards it.
 */

type ErrorDetail = components['schemas']['ErrorDetail'];

const UNSHAPED: ErrorDetail = { code: 'INTERNAL_ERROR', message: 'An unexpected error occurred' };

function detailOf(body: unknown): ErrorDetail {
	const shaped = (body as { error?: ErrorDetail } | undefined)?.error;
	return shaped?.code ? shaped : UNSHAPED;
}

export const DELETE: RequestHandler = async (event) => {
	const id = encodeURIComponent(event.params.id ?? '');

	let response: Response;

	try {
		response = await authenticatedFetch(event, `/api/blog/${id}/hard`, { method: 'DELETE' });
	} catch {
		return json({ error: UNSHAPED }, { status: 502 });
	}

	if (response.ok) return new Response(null, { status: 204 });

	const payload = await response.json().catch(() => null);
	return json({ error: detailOf(payload) }, { status: response.status });
};
