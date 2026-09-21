import type { PageServerLoad } from './$types';
import { authenticatedFetch } from '$lib/shared/api/backend.server';
import type { Tile } from '$lib/entities/media';

/**
 * `/studio/media` — one attachment target at a time.
 *
 * §01 refuses the combined view: "A combined library means four parallel calls
 * merged client-side with no paging — scope it as a per-target picker instead."
 * The target comes from the URL so a scoped library is a link somebody can
 * send, and the back button steps out of it.
 *
 * **A signed URL per ready image is a call per image.** Unlike a count column,
 * a grid of pictures cannot be drawn without them — so they are fetched here,
 * in parallel, rather than from the browser one tile at a time. Worth a batched
 * signing endpoint if this screen ever pages.
 */

const TARGETS = ['blog_post', 'project', 'resume', 'user'];

type Row = Tile & { attachment_target_id: string };

type Item = Tile & { src: string | null };

async function read<T>(
	event: Parameters<PageServerLoad>[0],
	path: string
): Promise<{ data?: T } | null> {
	try {
		const response = await authenticatedFetch(event, path);
		if (!response.ok) return null;

		return (await response.json()) as { data?: T };
	} catch {
		return null;
	}
}

/** A short-lived read URL. Resolved with the page, never stored. */
async function srcFor(
	event: Parameters<PageServerLoad>[0],
	mediaId: string
): Promise<string | null> {
	const body = await read<{ url?: string }>(
		event,
		`/api/media/${encodeURIComponent(mediaId)}/thumbnail`
	);

	return body?.data?.url ?? null;
}

export const load: PageServerLoad = async (event) => {
	const asked = event.url.searchParams.get('target');
	const scope = asked && TARGETS.includes(asked) ? asked : 'blog_post';

	const body = await read<{ rows?: Row[] }>(event, `/api/media/by-target/${scope}`);

	if (!body) return { items: [] as Item[], scope, failed: true };

	const rows = body.data?.rows ?? [];

	const items = await Promise.all(
		rows.map(async (row) => ({
			media_id: row.media_id,
			original_filename: row.original_filename,
			role: row.role,
			status: row.status,
			alt_text: row.alt_text,
			// Only what is ready has variants to point at.
			src: row.status === 'ready' ? await srcFor(event, row.media_id) : null
		}))
	);

	return { items, scope, failed: false };
};
