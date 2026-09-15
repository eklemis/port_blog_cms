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
 * Everything but topics pages, so a `per_page=1` call answers with the `total`
 * and one row — the blueprint's own note calls this fine at this scale and
 * worth replacing before it isn't. Applications joined that list after this
 * screen was written; reading the length of its first page would have reported
 * ten applications for anyone with more than ten. Topics still does not page,
 * so its count is the length of what comes back.
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

export type PostStates = { live: number; drafts: number; archived: number };

/**
 * How the posts divide, for the one tile sub-line that has something behind
 * it: "12 live · 9 drafts · 3 archived", from `GET /api/blog/summary`. Anything
 * but three numbers is no summary — a sub-line of zeros would say nothing is
 * live.
 */
async function postStates(event: Parameters<PageServerLoad>[0]): Promise<PostStates | null> {
	try {
		const response = await authenticatedFetch(event, '/api/blog/summary');
		if (!response.ok) return null;

		const body = (await response.json()) as { data?: Partial<Record<keyof PostStates, unknown>> };
		const { live, drafts, archived } = body.data ?? {};

		return typeof live === 'number' && typeof drafts === 'number' && typeof archived === 'number'
			? { live, drafts, archived }
			: null;
	} catch {
		return null;
	}
}

export const load: PageServerLoad = async (event) => {
	const [posts, projects, resumes, applications, topics, states] = await Promise.all([
		count(event, '/api/blog?per_page=1', fromTotal),
		count(event, '/api/projects?per_page=1', fromTotal),
		count(event, '/api/cvs?per_page=1', fromTotal),
		count(event, '/api/applications?per_page=1', fromTotal),
		count(event, '/api/topics', fromLength),
		postStates(event)
	]);

	return { counts: { posts, projects, resumes, applications, topics, postStates: states } };
};
