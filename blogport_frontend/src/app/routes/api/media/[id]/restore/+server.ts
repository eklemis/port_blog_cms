import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { authenticatedFetch } from '$lib/shared/api/backend.server';

/**
 * `POST /api/media/{id}/restore` — the middle rung of the archive ladder.
 *
 * It exists and nothing can reach it yet: `by-target` cannot be asked for
 * archived rows, so an archived image can be restored and not found. The route
 * is here because the ladder is, and because the day the listing can say
 * `deleted_at` this is the half that would otherwise be missing.
 */

export const POST: RequestHandler = async (event) => {
	const path = `/api/media/${encodeURIComponent(event.params.id ?? '')}/restore`;

	try {
		const response = await authenticatedFetch(event, path, { method: 'POST' });
		const payload = await response.json().catch(() => null);

		if (!response.ok) {
			return json(
				{ error: (payload as { error?: unknown })?.error ?? null },
				{
					status: response.status
				}
			);
		}

		return json({ data: (payload as { data?: unknown })?.data ?? null });
	} catch {
		return json({ error: { code: 'INTERNAL_ERROR' } }, { status: 502 });
	}
};
