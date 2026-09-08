import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { authenticatedFetch } from '$lib/shared/api/backend.server';

/**
 * `GET /api/blog/slug-available`.
 *
 * The backend answers with a free variant when the candidate is taken, so the
 * "-2" J4 asks the UI to suggest is the API's answer rather than a guess made
 * here — and it is an address that is actually free, which a guess is not.
 *
 * Every failure answers "available". This check is a courtesy ahead of the real
 * answer, which is `SLUG_ALREADY_EXISTS` on create; a check that reported
 * "taken" because it could not reach the backend would stop someone submitting
 * a slug that was free all along.
 */

type Availability = { available: boolean; slug: string; suggestion: string | null };

const unknown = (slug: string): Availability => ({ available: true, slug, suggestion: null });

export const GET: RequestHandler = async (event) => {
	const slug = (event.url.searchParams.get('slug') ?? '').trim();

	// Nothing to ask about, and the backend would answer about the empty string.
	if (!slug) return json({ available: false, slug: '', suggestion: null } satisfies Availability);

	const query = new URLSearchParams({ slug });

	try {
		const response = await authenticatedFetch(event, `/api/blog/slug-available?${query}`);
		if (!response.ok) return json(unknown(slug));

		const body = (await response.json()) as { data?: Partial<Availability> };
		const data = body.data;

		if (typeof data?.available !== 'boolean') return json(unknown(slug));

		return json({
			available: data.available,
			slug: data.slug ?? slug,
			suggestion: data.suggestion ?? null
		} satisfies Availability);
	} catch {
		return json(unknown(slug));
	}
};
