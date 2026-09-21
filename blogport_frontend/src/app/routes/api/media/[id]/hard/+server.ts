import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { authenticatedFetch } from '$lib/shared/api/backend.server';

/**
 * `DELETE /api/media/{id}/hard` — the rung that does not come back.
 *
 * "Deletes the media, attachment and variant rows. The stored objects are not
 * removed — reclaiming those is the bucket's lifecycle policy."
 *
 * Nothing calls it yet. Purge is offered on archived tiles, and archived tiles
 * cannot be listed; when they can, this is the call behind the second of the
 * two actions.
 */

export const DELETE: RequestHandler = async (event) => {
	const path = `/api/media/${encodeURIComponent(event.params.id ?? '')}/hard`;

	try {
		const response = await authenticatedFetch(event, path, { method: 'DELETE' });
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
