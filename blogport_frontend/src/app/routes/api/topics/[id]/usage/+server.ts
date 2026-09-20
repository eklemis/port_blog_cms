import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { authenticatedFetch } from '$lib/shared/api/backend.server';

/**
 * `GET /api/topics/{id}/usage` — what a retire confirmation needs.
 *
 * §02: "Count it first. Never drop a topic off eight pages silently." The
 * endpoint exists for this sentence; before it, "the console either warned
 * generically or invented a figure."
 *
 * A failure answers with no data rather than zeros. Zero reads as "nothing is
 * using it" and invites a confident retire, which is the mistake being avoided.
 */

export const GET: RequestHandler = async (event) => {
	const path = `/api/topics/${encodeURIComponent(event.params.id ?? '')}/usage`;

	try {
		const response = await authenticatedFetch(event, path);
		if (!response.ok) return json({ data: null }, { status: response.status });

		const payload = (await response.json()) as { data?: unknown };

		return json({ data: payload.data ?? null });
	} catch {
		return json({ data: null }, { status: 502 });
	}
};
