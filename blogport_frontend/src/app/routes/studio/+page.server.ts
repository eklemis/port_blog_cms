import type { PageServerLoad } from './$types';
import { authenticatedFetch } from '$lib/shared/api/backend.server';

/**
 * The Overview's counts.
 *
 * The four headline numbers the frame draws, and the topic count the first-run
 * checklist needs to know whether its first item is already done. They are
 * fetched together rather than in series: five round trips one after another is
 * five times the wait for a screen that is mostly numbers.
 *
 * Blog, projects and CVs page, so a `per_page=1` call answers with the `total`
 * and one row — the blueprint's own note calls this fine at this scale and
 * worth replacing before it isn't. Applications and topics do not page at all,
 * so their count is the length of the list they return.
 *
 * `GET /api/topics` is not in the map's row for this screen. It is listed for
 * the posts list, and the checklist the same row sanctions cannot say whether
 * "create a topic" is done without it — reported to the designer as an
 * incomplete row rather than resolved here. Nothing else on this screen reads
 * it, and a ticked box we cannot substantiate is the thing worth avoiding.
 */

/** `null` when the count could not be had: a stat tile must not invent a zero. */
async function count(
	event: Parameters<PageServerLoad>[0],
	path: string,
	read: (data: unknown) => number | null
): Promise<number | null> {
	try {
		const response = await authenticatedFetch(event, path);
		if (!response.ok) return null;

		const body = (await response.json()) as { data?: unknown };
		return read(body.data);
	} catch {
		return null;
	}
}

/** A paginated envelope: `total` counts every row, not just the page. */
const fromTotal = (data: unknown) => {
	const total = (data as { total?: unknown } | undefined)?.total;
	return typeof total === 'number' ? total : null;
};

/** An unparameterised list: what came back is all of it. */
const fromLength = (data: unknown) => (Array.isArray(data) ? data.length : null);

export const load: PageServerLoad = async (event) => {
	const [posts, projects, resumes, applications, topics] = await Promise.all([
		count(event, '/api/blog?per_page=1', fromTotal),
		count(event, '/api/projects?per_page=1', fromTotal),
		count(event, '/api/cvs?per_page=1', fromTotal),
		count(event, '/api/applications', fromLength),
		count(event, '/api/topics', fromLength)
	]);

	return { counts: { posts, projects, resumes, applications, topics } };
};
