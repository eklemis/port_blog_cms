import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { authenticatedFetch } from '$lib/shared/api/backend.server';
import type { components } from '$lib/shared/api/v1';

/**
 * `PATCH` and `DELETE /api/topics/{id}` — rename, and retire.
 *
 * The rename is the endpoint §02's journey does not know about: "Topics
 * supported create, list and soft delete only, so a typo in a title was
 * permanent and visible on every tagged post and project."
 *
 * `DELETE` is a soft delete — the row stays so existing references keep
 * resolving — which is why the console calls it retiring.
 *
 * A topic owned by somebody else is refused as forbidden rather than hidden as
 * not-found, and that distinction is passed through: `FORBIDDEN` and
 * `TOPIC_NOT_FOUND` are different classes in §07's table and lead somewhere
 * different.
 */

type ErrorDetail = components['schemas']['ErrorDetail'];

const UNSHAPED: ErrorDetail = { code: 'INTERNAL_ERROR', message: 'An unexpected error occurred' };

function detailOf(body: unknown): ErrorDetail {
	const shaped = (body as { error?: ErrorDetail } | undefined)?.error;
	return shaped?.code ? shaped : UNSHAPED;
}

async function relay(
	event: Parameters<RequestHandler>[0],
	method: 'PATCH' | 'DELETE'
): Promise<Response> {
	const path = `/api/topics/${encodeURIComponent(event.params.id ?? '')}`;
	const body = method === 'PATCH' ? await event.request.text() : undefined;

	let response: Response;

	try {
		response = await authenticatedFetch(event, path, {
			method,
			...(body === undefined ? {} : { headers: { 'content-type': 'application/json' }, body })
		});
	} catch {
		return json({ error: UNSHAPED }, { status: 502 });
	}

	const payload = await response.json().catch(() => null);

	if (!response.ok) return json({ error: detailOf(payload) }, { status: response.status });

	return json({ data: (payload as { data?: unknown })?.data ?? null });
}

export const PATCH: RequestHandler = (event) => relay(event, 'PATCH');

export const DELETE: RequestHandler = (event) => relay(event, 'DELETE');
