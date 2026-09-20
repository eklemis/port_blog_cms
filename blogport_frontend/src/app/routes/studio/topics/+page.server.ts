import type { PageServerLoad } from './$types';
import { authenticatedFetch } from '$lib/shared/api/backend.server';
import type { Topic } from '$lib/entities/topic';

/**
 * `/studio/topics` — the whole vocabulary on one screen.
 *
 * `GET /api/topics` returns the caller's own topics as a bare array rather
 * than a paged envelope, which is worth saying because the shape differs from
 * every other listing in the console.
 *
 * Usage counts are not fetched here. §02 asks for them in the list and for a
 * sort by usage, from a line written before `GET /api/topics/{id}/usage`
 * existed — one call per row is the same N+1 that §02 itself refused for the
 * posts table's Topics column. The counts are fetched when one is needed, at
 * the moment of retiring.
 */

export const load: PageServerLoad = async (event) => {
	try {
		const response = await authenticatedFetch(event, '/api/topics');
		if (!response.ok) return { topics: [] as Topic[], failed: true };

		const body = (await response.json()) as { data?: { items?: Topic[] } | Topic[] };
		const data = body.data;
		const items = Array.isArray(data) ? data : (data?.items ?? []);

		return { topics: items.filter((topic) => topic?.id), failed: false };
	} catch {
		return { topics: [] as Topic[], failed: true };
	}
};
