import type { PageServerLoad } from './$types';
import { authenticatedFetch } from '$lib/shared/api/backend.server';
import type { components } from '$lib/shared/api/v1';
import { coverOf, type Attachment } from '$lib/entities/media';

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

/**
 * The post's cover, and a URL to read it with.
 *
 * Two calls, because neither endpoint can do it alone.
 * `GET /api/media/by-target/blog_post?target_id=…&role=cover` asks for this
 * post's cover and nothing else. The parameters shipped as "Let a caller ask
 * for one thing's media", and this used to fetch every image on every post the
 * author had ever written and narrow it here — a comment in this file claimed
 * the parameters did not exist, hours after they did.
 *
 * `BlogPostResponse` still has no `cover` field, where `BlogPostCardResponse`
 * does, so it is still two calls. One narrow call is the part that matters.
 *
 * The read URL is short-lived by design, which is why it is resolved with the
 * page rather than stored: a signed URL written into anything outlives its own
 * validity.
 *
 * Its failure is not the page's. Losing this costs the rail its picture, so it
 * answers with none rather than taking the editor down with it.
 */
async function coverFor(
	event: Parameters<PageServerLoad>[0],
	postId: string
): Promise<{ cover: Attachment | null; coverSrc: string | null }> {
	const none = { cover: null, coverSrc: null };

	try {
		const query = new URLSearchParams({ target_id: postId, role: 'cover' });
		const response = await authenticatedFetch(event, `/api/media/by-target/blog_post?${query}`);
		if (!response.ok) return none;

		const body = (await response.json()) as { data?: { rows?: Attachment[] } };
		// Still narrowed here: asking for one post's cover should return one row,
		// and a loader that trusts that is a loader that breaks on the day it
		// does not.
		const cover = coverOf(body.data?.rows ?? [], postId);

		// A cover that is still processing is still the cover. It has no variants
		// to read yet, and the card says so rather than showing a gap.
		if (!cover || cover.status !== 'ready') return { cover, coverSrc: null };

		const variant = await authenticatedFetch(
			event,
			`/api/media/${encodeURIComponent(cover.media_id)}/medium`
		);
		if (!variant.ok) return { cover, coverSrc: null };

		const read = (await variant.json()) as { data?: { url?: string } };

		return { cover, coverSrc: read.data?.url ?? null };
	} catch {
		return none;
	}
}

export const load: PageServerLoad = async (event) => {
	const id = encodeURIComponent(event.params.id ?? '');
	const none: Topic[] = [];
	const noCover = { cover: null, coverSrc: null };

	try {
		const response = await authenticatedFetch(event, `/api/blog/${id}`);
		if (!response.ok) return { post: null, denied: true, availableTopics: none, ...noCover };

		const body = (await response.json()) as { data?: Detail };
		if (!body.data?.id) return { post: null, denied: true, availableTopics: none, ...noCover };

		const [availableTopics, cover] = await Promise.all([
			myTopics(event),
			coverFor(event, body.data.id)
		]);

		return { post: body.data, denied: false, availableTopics, ...cover };
	} catch {
		// Unreachable is indistinguishable from refused from here, and the
		// refusal is the safer thing to show: it offers a way back.
		return { post: null, denied: true, availableTopics: none, ...noCover };
	}
};
