import type { PageServerLoad } from './$types';
import { authenticatedFetch } from '$lib/shared/api/backend.server';
import type { components } from '$lib/shared/api/v1';

/**
 * `/studio/posts/archive` — archived posts, restore or purge.
 *
 * The same listing as the posts screen with `deleted=true`, which the backend
 * added for this screen: until then the owner query hard-coded live posts, and
 * this route had two actions and nothing to run them on.
 */

type Card = components['schemas']['BlogPostCardResponse'];

/** Blueprint §06: the default every list shares. */
const PER_PAGE = 10;

function pageFrom(raw: string | null): number {
	const value = Number(raw);
	return Number.isInteger(value) && value > 0 ? value : 1;
}

export const load: PageServerLoad = async (event) => {
	const page = pageFrom(event.url.searchParams.get('page'));
	const query = new URLSearchParams({
		deleted: 'true',
		page: String(page),
		per_page: String(PER_PAGE)
	});

	const empty = { posts: [] as Card[], total: 0, page, perPage: PER_PAGE };

	try {
		const response = await authenticatedFetch(event, `/api/blog?${query}`);
		if (!response.ok) return { ...empty, failed: true };

		const body = (await response.json()) as {
			data?: { items?: Card[]; total?: number; page?: number; per_page?: number };
		};

		return {
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
