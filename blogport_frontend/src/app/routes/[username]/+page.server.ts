import { error } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';
import { backendBaseUrl } from '$lib/shared/config/backend';
import { listedLabel } from '$lib/entities/post';
import type { components } from '$lib/shared/api/v1';

/**
 * `/[username]` — the front door, and the address the register and account
 * screens promise.
 *
 * Three calls, and they are not equal. The profile **is** the page: without it
 * there is nobody to introduce, so its failure is the page's. The two strips
 * are what the page points at, and either can come back empty without taking
 * the door down with it — §03 calls this "a doorway, not a fourth list".
 */

type PublicMedia = components['schemas']['PublicMedia'];

type Profile = {
	username: string;
	full_name?: string | null;
	bio?: string | null;
	avatar?: PublicMedia | null;
};

type Post = { slug: string; title: string; published_at?: string | null };

type Project = {
	slug: string;
	title: string;
	description?: string | null;
	tech_stack?: string[];
	cover?: PublicMedia | null;
};

type Paged<T> = { items?: T[] };

/** A doorway shows a few. Asking for ten and drawing three wastes seven. */
const SHOWN = 3;

const NO_SUCH_PAGE = "That page doesn't exist.";
const BROKE = 'Something went wrong on our side.';

const COVER_SIZES = ['large', 'medium', 'small', 'thumbnail'];
const AVATAR_SIZES = ['small', 'medium', 'thumbnail', 'large'];

function absolute(path: string): string {
	return /^https?:/i.test(path) ? path : `${backendBaseUrl}${path}`;
}

function imageOf(media: PublicMedia | null | undefined, sizes: string[]) {
	if (!media) return null;

	const size = sizes.find((name) => media.variants?.[name]);
	return size ? { src: absolute(media.variants[size]), alt: media.alt_text ?? '' } : null;
}

/** The profile: its failure is the page's. */
async function profileOf(event: Parameters<PageServerLoad>[0], username: string): Promise<Profile> {
	let response: Response;

	try {
		response = await event.fetch(`${backendBaseUrl}/api/public/users/${username}`);
	} catch {
		error(500, BROKE);
	}

	if (response.status === 404) error(404, NO_SUCH_PAGE);
	if (!response.ok) error(500, BROKE);

	const body = (await response.json().catch(() => null)) as { data?: Profile } | null;
	if (!body?.data) error(500, BROKE);

	return body.data;
}

/** A strip: its failure is one door fewer, and the page still stands. */
async function strip<T>(event: Parameters<PageServerLoad>[0], path: string): Promise<T[]> {
	try {
		const response = await event.fetch(`${backendBaseUrl}${path}`);
		if (!response.ok) return [];

		const body = (await response.json().catch(() => null)) as { data?: Paged<T> } | null;
		return body?.data?.items ?? [];
	} catch {
		return [];
	}
}

export const load: PageServerLoad = async (event) => {
	const username = encodeURIComponent(event.params.username ?? '');
	const few = new URLSearchParams({ page: '1', per_page: String(SHOWN) });

	const [profile, posts, projects] = await Promise.all([
		profileOf(event, username),
		strip<Post>(event, `/api/public/blog/${username}?${few}`),
		strip<Project>(event, `/api/public/projects/${username}?${few}`)
	]);

	return {
		author: {
			username: profile.username,
			fullName: profile.full_name?.trim() || profile.username,
			bio: profile.bio ?? null,
			avatarSrc: imageOf(profile.avatar, AVATAR_SIZES)?.src ?? null
		},
		posts: posts.slice(0, SHOWN).map((post) => ({
			slug: post.slug,
			title: post.title,
			published: listedLabel(post.published_at)
		})),
		projects: projects.slice(0, SHOWN).map((project) => ({
			slug: project.slug,
			title: project.title,
			description: project.description ?? null,
			techStack: project.tech_stack ?? [],
			cover: imageOf(project.cover, COVER_SIZES)
		}))
	};
};
