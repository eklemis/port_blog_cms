import type { PageServerLoad } from './$types';
import { authenticatedFetch } from '$lib/shared/api/backend.server';
import type { components } from '$lib/shared/api/v1';

/**
 * `/studio/posts` — the list.
 *
 * Every filter is read from the URL and written back to it, so a filtered list
 * is a shareable, reloadable, back-button-safe address rather than something
 * that only exists in one tab's memory. Defaults are page=1, per_page=10 and
 * the resource's own sort.
 */

type Card = components['schemas']['BlogPostCardResponse'];
type Sort = components['schemas']['BlogPostSort'];

/** Blueprint §06: the default every list shares. */
const PER_PAGE = 10;

const SORTS: Sort[] = ['newest', 'oldest', 'published_newest', 'updated_newest'];

function sortFrom(raw: string | null): Sort | null {
	return SORTS.includes(raw as Sort) ? (raw as Sort) : null;
}

function pageFrom(raw: string | null): number {
	const page = Number(raw);
	return Number.isInteger(page) && page > 0 ? page : 1;
}

export const load: PageServerLoad = async (event) => {
	const params = event.url.searchParams;

	const search = (params.get('search') ?? '').trim();
	const published = params.get('published');
	const sort = sortFrom(params.get('sort'));
	const page = pageFrom(params.get('page'));

	// What makes empty different from filtered-empty. Conflating the two tells
	// someone with 24 posts that they have none.
	const filtered = Boolean(search) || published === 'true' || published === 'false';

	const query = new URLSearchParams({ page: String(page), per_page: String(PER_PAGE) });
	if (search) query.set('search', search);
	if (published === 'true' || published === 'false') query.set('published', published);
	if (sort) query.set('sort', sort);

	const empty = {
		posts: [] as Card[],
		total: 0,
		page,
		perPage: PER_PAGE,
		filtered,
		search,
		published,
		sort
	};

	try {
		const response = await authenticatedFetch(event, `/api/blog?${query}`);
		if (!response.ok) return { ...empty, failed: true };

		const body = (await response.json()) as {
			data?: { items?: Card[]; total?: number; page?: number; per_page?: number };
		};

		return {
			...empty,
			posts: body.data?.items ?? [],
			total: body.data?.total ?? 0,
			page: body.data?.page ?? page,
			perPage: body.data?.per_page ?? PER_PAGE,
			failed: false
		};
	} catch {
		return { ...empty, failed: true };
	}
};
