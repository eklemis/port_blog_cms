import { error } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';
import { backendBaseUrl } from '$lib/shared/config/backend';
import type { components } from '$lib/shared/api/v1';

/**
 * `/[username]/projects` — an author's projects.
 *
 * Public and sessionless, server-rendered like every public route. The shape of
 * this loader is the blog index's, because the two screens differ in what they
 * draw and not in how they are fetched: a listing, a profile for the header,
 * and one 404 for everything missing.
 */

type PublicMedia = components['schemas']['PublicMedia'];

type Card = {
	slug: string;
	title: string;
	description?: string | null;
	tech_stack?: string[];
	topics?: { id: string; title: string }[];
	repo_url?: string | null;
	live_demo_url?: string | null;
	cover?: PublicMedia | null;
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

/** §03: never confirm which usernames exist. */
const NO_SUCH_PAGE = "That page doesn't exist.";

/** The blueprint's INTERNAL_ERROR copy. */
const BROKE = 'Something went wrong on our side.';

const COVER_SIZES = ['large', 'medium', 'small', 'thumbnail'];
const AVATAR_SIZES = ['thumbnail', 'small', 'medium', 'large'];

function absolute(path: string): string {
	return /^https?:/i.test(path) ? path : `${backendBaseUrl}${path}`;
}

function imageOf(media: PublicMedia | null | undefined, sizes: string[]) {
	if (!media) return null;

	// A media row exists before its variants do.
	const size = sizes.find((name) => media.variants?.[name]);
	if (!size) return null;

	return { src: absolute(media.variants[size]), alt: media.alt_text ?? '' };
}

async function read<T>(event: Parameters<PageServerLoad>[0], path: string): Promise<T> {
	let response: Response;

	try {
		response = await event.fetch(`${backendBaseUrl}${path}`);
	} catch {
		error(500, BROKE);
	}

	// A 404 is an answer about the author; anything else is an answer about us.
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
		read<Paged<Card>>(event, `/api/public/projects/${username}?${query}`),
		read<Profile>(event, `/api/public/users/${username}`)
	]);

	const items = listing.items ?? [];

	// Every card carries its own topics now, which is what lets the active chip
	// have a name. The row of chips to choose from still has no public source.
	const named = topicId
		? items.flatMap((project) => project.topics ?? []).find((topic) => topic.id === topicId)
		: undefined;

	return {
		author: {
			username: profile.username,
			fullName: profile.full_name?.trim() || profile.username,
			avatarSrc: imageOf(profile.avatar, AVATAR_SIZES)?.src ?? null
		},
		projects: items.map((project) => ({
			slug: project.slug,
			title: project.title,
			description: project.description ?? null,
			techStack: project.tech_stack ?? [],
			topics: project.topics ?? [],
			cover: imageOf(project.cover, COVER_SIZES),
			repoUrl: project.repo_url ?? null,
			demoUrl: project.live_demo_url ?? null
		})),
		filter: topicId ? { id: topicId, title: named?.title ?? 'this topic' } : null,
		total: listing.total ?? items.length,
		page: listing.page ?? wanted,
		perPage: listing.per_page ?? PER_PAGE
	};
};
