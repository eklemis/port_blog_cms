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
 * **A signed URL per ready image is a call per image**, and the calls are
 * bounded by what the page renders — which is the line worth keeping: a count
 * column was information the screen survived without, and unbounded besides.
 * A grid of pictures cannot be drawn without the URLs. Worth a batched signing
 * endpoint if this screen ever pages.
 *
 * An archived row is not signed for. It is drawn as a marked tile rather than a
 * picture, so a URL for it would be a request for something never shown.
 */

const TARGETS = ['blog_post', 'project', 'resume', 'user'];

type Row = Tile & { attachment_target_id: string; deleted_at?: string | null };

type Item = Tile & { src: string | null; deleted_at: string | null };

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
	const archived = event.url.searchParams.get('archived') === 'true';

	// One grid, as the frame draws it: archived rows come back alongside the
	// live ones and `deleted_at` tells them apart.
	const query = archived ? '?include_deleted=true' : '';
	const body = await read<{ rows?: Row[] }>(event, `/api/media/by-target/${scope}${query}`);

	if (!body) return { items: [] as Item[], scope, archived, failed: true };

	const rows = body.data?.rows ?? [];

	const items = await Promise.all(
		rows.map(async (row) => ({
			media_id: row.media_id,
			original_filename: row.original_filename,
			role: row.role,
			status: row.status,
			alt_text: row.alt_text,
			deleted_at: row.deleted_at ?? null,
			// Only what is ready and still live has variants worth signing for.
			src: row.status === 'ready' && !row.deleted_at ? await srcFor(event, row.media_id) : null
		}))
	);

	return { items, scope, archived, failed: false };
};
