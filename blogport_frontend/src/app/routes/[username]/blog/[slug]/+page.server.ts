import { error } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';
import { backendBaseUrl } from '$lib/shared/config/backend';
import { readingTime } from '$lib/entities/post';
import { renderMarkdown } from '$lib/shared/lib/markdown.server';
import type { components } from '$lib/shared/api/v1';

/**
 * `/[username]/blog/[slug]` — a published post, read by a stranger.
 *
 * Public and deliberately sessionless. §03: every public route is
 * server-rendered "so a shared link is a real page with real metadata — not a
 * shell that fetches after paint". The body becomes HTML here for the same
 * reason, and because a reader should not have to download a parser to read an
 * article.
 *
 * Everything that is missing answers the same 404 — a post that was never
 * written, one still in draft, one scheduled for next week, one archived, and a
 * username nobody has. That is the point: §03 says never to confirm which posts
 * or usernames exist, and a route that distinguishes them is a route that tells.
 */

type PublicMedia = components['schemas']['PublicMedia'];

type Post = components['schemas']['BlogPostResponse'] & {
	cover?: PublicMedia | null;
	topics?: { id: string; title: string }[];
};

type Profile = {
	username: string;
	full_name?: string | null;
	bio?: string | null;
	avatar?: PublicMedia | null;
};

/** The spec's copy, and the only thing this route ever says went wrong. */
const NOT_FOUND = "That post doesn't exist, or it isn't published yet.";

/** Biggest first: the cover is 700px wide before it is anything else. */
const COVER_SIZES = ['large', 'medium', 'small', 'thumbnail'];

/**
 * A public media path, made absolute.
 *
 * Public responses carry a stable path that redirects to a freshly signed URL
 * (ADR 0006), so it is safe in a cached page — but it is the *backend's* path,
 * and a relative one would be asked of this app instead.
 */
function absolute(path: string): string {
	return /^https?:/i.test(path) ? path : `${backendBaseUrl}${path}`;
}

function coverOf(media: PublicMedia | null | undefined) {
	if (!media) return null;

	// A media row exists before its variants do, so a post published while its
	// cover is still processing has the attachment and no URL behind it.
	const size = COVER_SIZES.find((name) => media.variants?.[name]);
	if (!size) return null;

	return { src: absolute(media.variants[size]), alt: media.alt_text ?? '' };
}

async function read<T>(event: Parameters<PageServerLoad>[0], path: string): Promise<T> {
	let response: Response;

	try {
		response = await event.fetch(`${backendBaseUrl}${path}`);
	} catch {
		// Unreachable and gone are the same thing to somebody holding a link.
		error(404, NOT_FOUND);
	}

	if (!response.ok) error(404, NOT_FOUND);

	const body = (await response.json().catch(() => null)) as { data?: T } | null;
	if (!body?.data) error(404, NOT_FOUND);

	return body.data;
}

export const load: PageServerLoad = async (event) => {
	const username = encodeURIComponent(event.params.username ?? '');
	const slug = encodeURIComponent(event.params.slug ?? '');

	// Both at once: the post is the page and the profile is its header, and
	// neither is worth a round trip's wait on the other.
	const [post, profile] = await Promise.all([
		read<Post>(event, `/api/public/blog/${username}/${slug}`),
		read<Profile>(event, `/api/public/users/${username}`)
	]);

	const bodyHtml = renderMarkdown(
		post.content ?? '',
		(mediaId) => `${backendBaseUrl}/api/public/media/${encodeURIComponent(mediaId)}/large`
	);

	return {
		author: {
			username: profile.username,
			// An author who never set a display name is still somebody; their
			// handle is the name they have.
			fullName: profile.full_name?.trim() || profile.username,
			avatarSrc: coverOf(profile.avatar)?.src ?? null
		},
		post: {
			title: post.title,
			excerpt: post.excerpt ?? null,
			publishedAt: post.published_at ?? null,
			slug: post.slug
		},
		topics: post.topics ?? [],
		cover: coverOf(post.cover),
		bodyHtml,
		readMinutes: readingTime(post.content ?? '')
	};
};
