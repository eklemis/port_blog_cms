import type { PageServerLoad } from './$types';
import { authenticatedFetch } from '$lib/shared/api/backend.server';
import type { components } from '$lib/shared/api/v1';

/**
 * `/studio/posts/[id]` — the editor's post.
 *
 * `GET /api/blog/{id}` returns drafts as well as published posts, together with
 * their topics. A post belonging to another author comes back as not found
 * rather than forbidden, so both outcomes are the same screen: J4 asks for a
 * plain sentence and a route back to the list, never a bounce through login.
 */

type Detail = components['schemas']['BlogPostResponse'];

type Topic = { id: string; title: string };

/**
 * The author's own topics, for the rail's picker to offer — §03, "own topics
 * only".
 *
 * Its failure is not the page's. Losing this list costs the picker its options
 * and nothing else, so it answers with none rather than taking the editor down
 * with it.
 */
async function myTopics(event: Parameters<PageServerLoad>[0]): Promise<Topic[]> {
	try {
		const response = await authenticatedFetch(event, '/api/topics');
		if (!response.ok) return [];

		const body = (await response.json()) as { data?: { items?: Topic[] } | Topic[] };
		const data = body.data;
		const items = Array.isArray(data) ? data : (data?.items ?? []);

		return items.filter((topic) => topic?.id);
	} catch {
		return [];
	}
}

export const load: PageServerLoad = async (event) => {
	const id = encodeURIComponent(event.params.id ?? '');
	const none: Topic[] = [];

	try {
		const response = await authenticatedFetch(event, `/api/blog/${id}`);
		if (!response.ok) return { post: null, denied: true, availableTopics: none };

		const body = (await response.json()) as { data?: Detail };
		if (!body.data?.id) return { post: null, denied: true, availableTopics: none };

		return { post: body.data, denied: false, availableTopics: await myTopics(event) };
	} catch {
		// Unreachable is indistinguishable from refused from here, and the
		// refusal is the safer thing to show: it offers a way back.
		return { post: null, denied: true, availableTopics: none };
	}
};
