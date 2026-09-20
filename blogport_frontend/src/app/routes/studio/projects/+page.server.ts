import type { PageServerLoad } from './$types';
import { authenticatedFetch } from '$lib/shared/api/backend.server';
import type { ProjectCard } from '$lib/entities/project';

/**
 * `/studio/projects` — Screen / Projects list 70:2.
 *
 * Three calls: the page of rows, the topics the filter offers, and the
 * unfiltered count that the filtered-empty state needs. Only the first can fail
 * the page; the other two cost a control and a sentence.
 */

const PER_PAGE = 10;

type Topic = { id: string; title: string };

type Page = { items: ProjectCard[]; total: number; page: number; per_page: number };

async function read<T>(
	event: Parameters<PageServerLoad>[0],
	path: string
): Promise<{ data?: T } | null> {
	try {
		const response = await authenticatedFetch(event, path);
		if (!response.ok) return null;

		return (await response.json()) as { data?: T };
	} catch {
		return null;
	}
}

/**
 * The author's own topics, for the filter.
 *
 * Its failure is not the page's: losing it costs the filter its options and
 * nothing else, so it answers with none rather than taking the list down.
 */
async function myTopics(event: Parameters<PageServerLoad>[0]): Promise<Topic[]> {
	const body = await read<{ items?: Topic[] } | Topic[]>(event, '/api/topics');
	const data = body?.data;
	const items = Array.isArray(data) ? data : (data?.items ?? []);

	return items.filter((topic) => topic?.id);
}

export const load: PageServerLoad = async (event) => {
	const params = event.url.searchParams;
	const search = params.get('search');
	const topic = params.get('topic_id');
	const sort = params.get('sort');
	const page = Number(params.get('page')) || 1;
	const filtered = Boolean(search || topic);

	const query = new URLSearchParams({ page: String(page), per_page: String(PER_PAGE) });
	if (search) query.set('search', search);
	if (topic) query.set('topic_id', topic);
	if (sort) query.set('sort', sort);

	const [body, topics] = await Promise.all([
		read<Page>(event, `/api/projects?${query}`),
		myTopics(event)
	]);

	const empty = {
		projects: [] as ProjectCard[],
		total: 0,
		page,
		perPage: PER_PAGE,
		filtered,
		search: search ?? '',
		topic,
		sort,
		topics,
		everything: null as number | null
	};

	if (!body) return { ...empty, failed: true };

	/**
	 * The unfiltered count, for the sentence that tells someone with four
	 * projects that four exist and none match — rather than that they have none.
	 * Only asked for when a filter is on, and only worth one extra call.
	 */
	const everything = filtered
		? ((await read<Page>(event, '/api/projects?page=1&per_page=1'))?.data?.total ?? null)
		: (body.data?.total ?? 0);

	return {
		...empty,
		projects: body.data?.items ?? [],
		total: body.data?.total ?? 0,
		page: body.data?.page ?? page,
		perPage: body.data?.per_page ?? PER_PAGE,
		everything,
		failed: false
	};
};
