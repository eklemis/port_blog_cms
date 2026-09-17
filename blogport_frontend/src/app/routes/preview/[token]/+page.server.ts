import { error } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';
import { backendBaseUrl } from '$lib/shared/config/backend';
import type { components } from '$lib/shared/api/v1';

/**
 * `/preview/[token]` — a draft read through its share link.
 *
 * Public, and deliberately sessionless: the token is the authorisation, so
 * this calls the public endpoint with the caller's own fetch and no cookies.
 * Anyone holding the link sees exactly what a reviewer sees.
 *
 * Unknown, revoked and expired all answer 404 — the same answer on purpose, so
 * the endpoint cannot be used to find out which posts exist. Nothing here tries
 * to tell them apart either.
 */

type Preview = components['schemas']['BlogPostResponse'] & {
	topics?: { id: string; title: string }[];
	preview?: boolean;
};

export const load: PageServerLoad = async (event) => {
	const token = encodeURIComponent(event.params.token ?? '');

	let response: Response;

	try {
		response = await event.fetch(`${backendBaseUrl}/api/public/blog/preview/${token}`);
	} catch {
		// Nothing can be shown, and a reader holding a link cannot act on the
		// difference between "gone" and "unreachable".
		error(404, 'That preview link is no longer valid.');
	}

	if (!response.ok) error(404, 'That preview link is no longer valid.');

	const body = (await response.json().catch(() => null)) as { data?: Preview } | null;
	if (!body?.data?.id) error(404, 'That preview link is no longer valid.');

	return { post: body.data };
};
