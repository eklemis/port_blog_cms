import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { authenticatedFetch } from '$lib/shared/api/backend.server';
import type { components } from '$lib/shared/api/v1';

/**
 * `POST /api/blog/bulk` — one operation over up to a hundred posts.
 *
 * A 200 means the batch ran, not that every item did, so the outcome is
 * forwarded whole — `failed` included — rather than flattened into a yes or a
 * no. The archive screen keeps each failed row selected with its own reason.
 *
 * The body is forwarded as it came. `op` is flattened into the request and the
 * generated type does not describe it (see the PR), so there is nothing here
 * to check it against that the backend will not check better.
 */

type ErrorDetail = components['schemas']['ErrorDetail'];

const UNSHAPED: ErrorDetail = { code: 'INTERNAL_ERROR', message: 'An unexpected error occurred' };

function detailOf(body: unknown): ErrorDetail {
	const shaped = (body as { error?: ErrorDetail } | undefined)?.error;
	return shaped?.code ? shaped : UNSHAPED;
}

export const POST: RequestHandler = async (event) => {
	const body = await event.request.json();

	let response: Response;

	try {
		response = await authenticatedFetch(event, '/api/blog/bulk', {
			method: 'POST',
			headers: { 'content-type': 'application/json' },
			body: JSON.stringify(body)
		});
	} catch {
		return json({ error: UNSHAPED }, { status: 502 });
	}

	const payload = await response.json().catch(() => null);

	if (!response.ok) {
		return json({ error: detailOf(payload) }, { status: response.status });
	}

	return json((payload as { data?: unknown })?.data ?? null);
};
