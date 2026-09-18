import { error } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';
import { backendBaseUrl } from '$lib/shared/config/backend';
import { listedLabel } from '$lib/entities/post';
import type { components } from '$lib/shared/api/v1';

/**
 * `/[username]/blog` — an author's published posts.
 *
 * Public and sessionless, server-rendered like every public route so a shared
 * link is a real page rather than a shell that fetches after paint.
 *
 * Two calls. The route table's "Backed by" column names only the listing, but
 * the header needs the author's name, bio and avatar, and the frame says where
 * those come from (72:86). Reported: this is the fourth screen whose "Backed
 * by" row names the main call and not the supporting one.
 *
 * Filtering is the API's, not this page's. Both public listings take `topic_id`,
 * and filtering the ten rows already fetched would filter a page while claiming
 * to have filtered the list.
 */

type PublicMedia = components['schemas']['PublicMedia'];

type Listed = components['schemas']['BlogPostResponse'] & {
	cover?: PublicMedia | null;
	topics?: { id: string; title: string }[];
};

type Profile = {
	username: string;
	full_name?: string | null;
	bio?: string | null;
	avatar?: PublicMedia | null;
};

type Paged<T> = { items?: T[]; total?: number; page?: number; per_page?: number };

/** Blueprint §06: the default every list shares. */
const PER_PAGE = 10;

/**
 * There is no author here, and the page says so without saying who was asked
 * for. §03: never confirm which usernames exist.
 */
const NO_SUCH_PAGE = "That page doesn't exist.";

/** The blueprint's INTERNAL_ERROR copy. */
const BROKE = 'Something went wrong on our side.';

/** Biggest first. The avatar is drawn at 58px, so a thumbnail is enough. */
const AVATAR_SIZES = ['thumbnail', 'small', 'medium', 'large'];

function absolute(path: string): string {
	return /^https?:/i.test(path) ? path : `${backendBaseUrl}${path}`;
}

function avatarOf(media: PublicMedia | null | undefined): string | null {
	if (!media) return null;

	const size = AVATAR_SIZES.find((name) => media.variants?.[name]);
	return size ? absolute(media.variants[size]) : null;
}

/**
 * A 404 is an answer about the author; anything else is an answer about us.
 * Collapsing the two would tell a reader that somebody's page does not exist
 * because a database was briefly unreachable.
 */
async function read<T>(event: Parameters<PageServerLoad>[0], path: string): Promise<T> {
	let response: Response;

	try {
		response = await event.fetch(`${backendBaseUrl}${path}`);
	} catch {
		error(500, BROKE);
	}

	if (response.status === 404) error(404, NO_SUCH_PAGE);
	if (!response.ok) error(500, BROKE);

	const body = (await response.json().catch(() => null)) as { data?: T } | null;
	if (!body?.data) error(500, BROKE);

	return body.data;
}

function pageFrom(raw: string | null): number {
	const value = Number(raw);
	return Number.isInteger(value) && value > 0 ? value : 1;
}

export const load: PageServerLoad = async (event) => {
	const username = encodeURIComponent(event.params.username ?? '');
	const wanted = pageFrom(event.url.searchParams.get('page'));
	const topicId = event.url.searchParams.get('topic_id');

	const query = new URLSearchParams({ page: String(wanted), per_page: String(PER_PAGE) });
	if (topicId) query.set('topic_id', topicId);

	const [listing, profile] = await Promise.all([
		read<Paged<Listed>>(event, `/api/public/blog/${username}?${query}`),
		read<Profile>(event, `/api/public/users/${username}`)
	]);

	const items = listing.items ?? [];

	// The only public source for a topic's title. When the filter matches
	// nothing there is nothing to read it from, and the chip says what it can.
	const named = topicId
		? items.flatMap((post) => post.topics ?? []).find((topic) => topic.id === topicId)
		: undefined;

	return {
		author: {
			username: profile.username,
			fullName: profile.full_name?.trim() || profile.username,
			bio: profile.bio ?? null,
			avatarSrc: avatarOf(profile.avatar)
		},
		posts: items.map((post) => ({
			slug: post.slug,
			title: post.title,
			excerpt: post.excerpt ?? null,
			published: listedLabel(post.published_at),
			publishedAt: post.published_at ?? null
		})),
		filter: topicId ? { id: topicId, title: named?.title ?? 'this topic' } : null,
		total: listing.total ?? items.length,
		page: listing.page ?? wanted,
		perPage: listing.per_page ?? PER_PAGE
	};
};
