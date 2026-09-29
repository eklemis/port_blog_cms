import type { PageServerLoad } from './$types';
import { authenticatedFetch } from '$lib/shared/api/backend.server';

/**
 * `/studio/resumes` — Screen / Résumés 241:5528.
 *
 * `GET /api/cvs?search=&sort=&page=&per_page=`, as §03's surface map names it.
 * Searching and sorting go back to the server: a page of ten filtered in the
 * browser hides rows on this page and misses every match on the others.
 *
 * The counts in the Sections column come off the rows this call already
 * returns, so the cell costs no second request.
 */

const PER_PAGE = 10;

type Row = {
	id: string;
	role: string;
	bio: string;
	experiences: unknown[];
	highlighted_projects: unknown[];
};

type Page = { items: Row[]; total: number; page: number; per_page: number };

async function read(
	event: Parameters<PageServerLoad>[0],
	path: string
): Promise<{ data?: Page } | null> {
	try {
		const response = await authenticatedFetch(event, path);
		if (!response.ok) return null;

		return (await response.json()) as { data?: Page };
	} catch {
		return null;
	}
}

export const load: PageServerLoad = async (event) => {
	const params = event.url.searchParams;
	const search = params.get('search');
	const sort = params.get('sort');
	const page = Number(params.get('page')) || 1;
	const filtered = Boolean(search);

	const query = new URLSearchParams({ page: String(page), per_page: String(PER_PAGE) });
	if (search) query.set('search', search);
	if (sort) query.set('sort', sort);

	const body = await read(event, `/api/cvs?${query}`);

	const empty = {
		resumes: [] as Row[],
		total: 0,
		page,
		perPage: PER_PAGE,
		filtered,
		search: search ?? '',
		sort,
		everything: null as number | null
	};

	if (!body) return { ...empty, failed: true };

	/**
	 * The unfiltered count, for the sentence telling someone with four résumés
	 * that four exist and none match — rather than that they have none.
	 */
	const everything = filtered
		? ((await read(event, '/api/cvs?page=1&per_page=1'))?.data?.total ?? null)
		: (body.data?.total ?? 0);

	return {
		...empty,
		// Rows without an id cannot be opened, and would key the each block on
		// undefined.
		resumes: (body.data?.items ?? []).filter((row) => row?.id),
		total: body.data?.total ?? 0,
		page: body.data?.page ?? page,
		perPage: body.data?.per_page ?? PER_PAGE,
		everything,
		failed: false
	};
};
