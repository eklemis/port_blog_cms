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

export const load: PageServerLoad = async (event) => {
	const id = encodeURIComponent(event.params.id ?? '');

	try {
		const response = await authenticatedFetch(event, `/api/blog/${id}`);
		if (!response.ok) return { post: null, denied: true };

		const body = (await response.json()) as { data?: Detail };
		if (!body.data?.id) return { post: null, denied: true };

		return { post: body.data, denied: false };
	} catch {
		// Unreachable is indistinguishable from refused from here, and the
		// refusal is the safer thing to show: it offers a way back.
		return { post: null, denied: true };
	}
};
