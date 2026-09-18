import { error } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';
import { backendBaseUrl } from '$lib/shared/config/backend';
import { renderMarkdown } from '$lib/shared/lib/markdown.server';
import type { components } from '$lib/shared/api/v1';

/**
 * `/[username]/projects/[slug]` — one project.
 *
 * Public and sessionless. The description is markdown (§03's field table puts
 * it in that class, with `excerpt` the one exception that stays plain), so it
 * becomes HTML here for the same reasons a post's body does: a reader gets the
 * finished article, and stored text never reaches the page as markup.
 *
 * `owner` is a bare UUID, so the header's name and avatar come from the public
 * profile — a second call §03's "Backed by" column does not name.
 */

type PublicMedia = components['schemas']['PublicMedia'];

type Project = components['schemas']['ProjectView'];

type Profile = {
	username: string;
	full_name?: string | null;
	bio?: string | null;
	avatar?: PublicMedia | null;
};

const NO_SUCH_PAGE = "That page doesn't exist.";
const BROKE = 'Something went wrong on our side.';

const SIZES = ['large', 'medium', 'small', 'thumbnail'];
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

/**
 * The gallery, cover first.
 *
 * `media` and `screenshots` coexist: the first is uploaded and carries
 * generated sizes, the second is a list of addresses stored on the row. Media
 * wins when there is any, because it has alt text and real sizes; a project
 * with only the older field still gets its pictures.
 */
function galleryOf(project: Project) {
	const uploaded = (project.media ?? [])
		.slice()
		.sort((a, b) => {
			// Cover first, then whatever order the author put them in.
			if (a.role === 'cover' && b.role !== 'cover') return -1;
			if (b.role === 'cover' && a.role !== 'cover') return 1;
			return (a.position ?? 0) - (b.position ?? 0);
		})
		.map((media) => imageOf(media, SIZES))
		.filter((image): image is { src: string; alt: string } => image !== null);

	if (uploaded.length) return uploaded;

	return (project.screenshots ?? []).map((src) => ({ src, alt: '' }));
}

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

export const load: PageServerLoad = async (event) => {
	const username = encodeURIComponent(event.params.username ?? '');
	const slug = encodeURIComponent(event.params.slug ?? '');

	const [project, profile] = await Promise.all([
		read<Project>(event, `/api/public/projects/${username}/${slug}`),
		read<Profile>(event, `/api/public/users/${username}`)
	]);

	return {
		author: {
			username: profile.username,
			fullName: profile.full_name?.trim() || profile.username,
			avatarSrc: imageOf(profile.avatar, AVATAR_SIZES)?.src ?? null
		},
		project: {
			title: project.title,
			slug: project.slug,
			techStack: project.tech_stack ?? [],
			topics: (project.topics ?? []).map((topic) => ({ id: topic.id, title: topic.title })),
			repoUrl: project.repo_url ?? null,
			demoUrl: project.live_demo_url ?? null
		},
		bodyHtml: renderMarkdown(
			project.description ?? '',
			(mediaId) => `${backendBaseUrl}/api/public/media/${encodeURIComponent(mediaId)}/large`
		),
		images: galleryOf(project)
	};
};
