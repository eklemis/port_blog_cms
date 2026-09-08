import type { PageServerLoad } from './$types';
import { authenticatedFetch } from '$lib/shared/api/backend.server';

/**
 * The Overview's counts.
 *
 * Three totals, each from a paginated list fetched at `per_page=1` — the
 * blueprint's own note calls this fine at this scale and worth replacing before
 * it isn't. They are fetched together rather than in series: three round trips
 * one after another is three times the wait for a screen that is mostly numbers.
 *
 * Three, not the four the frame draws. The route map sanctions
 * `GET /api/blog · /api/projects · /api/cvs` for this screen and nothing else,
 * and "Applications" appears nowhere in the Console Blueprint — so the fourth
 * card, "Needs attention" and "Getting started" are not built. See the PR.
 */

/** `null` when the count could not be had: a stat tile must not invent a zero. */
async function total(event: Parameters<PageServerLoad>[0], path: string): Promise<number | null> {
	try {
		const response = await authenticatedFetch(event, `${path}?per_page=1`);
		if (!response.ok) return null;

		const body = (await response.json()) as { data?: { total?: unknown } };
		return typeof body.data?.total === 'number' ? body.data.total : null;
	} catch {
		return null;
	}
}

export const load: PageServerLoad = async (event) => {
	const [posts, projects, resumes] = await Promise.all([
		total(event, '/api/blog'),
		total(event, '/api/projects'),
		total(event, '/api/cvs')
	]);

	return { counts: { posts, projects, resumes } };
};
