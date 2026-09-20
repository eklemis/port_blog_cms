import type { PageServerLoad } from './$types';
import { authenticatedFetch } from '$lib/shared/api/backend.server';
import type { Attachment } from '$lib/entities/media';
import type { Topic } from '$lib/entities/topic';

/**
 * `/studio/projects/[id]` — the editor's project, its screenshots and the
 * topics the picker offers.
 *
 * A project belonging to another author comes back as not found rather than
 * forbidden, so both outcomes are the same screen: a plain sentence and a route
 * back to the list, never a bounce through login.
 *
 * Screenshots come from the media listing rather than from the project, which
 * carries only a `screenshots: string[]` of addresses with no ids, no filenames
 * and nothing to reorder by. The listing takes the target *kind* with no id and
 * no role, so this narrows it here — the same gap as the post editor's cover,
 * and the same backend ask.
 */

type Project = {
	id: string;
	title: string;
	slug: string;
	description?: string | null;
	tech_stack: string[];
	topics: Topic[];
	repo_url?: string | null;
	live_demo_url?: string | null;
};

type Shot = { media_id: string; original_filename: string; src?: string | null };

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

/**
 * This project's gallery, in the order the author put it in.
 *
 * Sorted by `position` — "display order within the role, starting at 0" — which
 * is the field that makes reordering possible at all. A screenshot still
 * processing is kept: it is part of the gallery, and the card shows its state
 * rather than a hole.
 */
async function screenshotsFor(
	event: Parameters<PageServerLoad>[0],
	projectId: string
): Promise<Shot[]> {
	// `position` and `original_filename` are on `MediaItem` but not on the
	// entity's narrowed `Attachment`, which exists for the cover card and needs
	// neither. Widened here rather than there, so the narrow type stays narrow.
	type Row = Attachment & { position?: number; original_filename?: string };

	const body = await read<{ rows?: Row[] }>(event, '/api/media/by-target/project');

	return (body?.data?.rows ?? [])
		.filter((row) => row.attachment_target_id === projectId && row.role !== 'cover')
		.slice()
		.sort((a, b) => (a.position ?? 0) - (b.position ?? 0))
		.map((row) => ({
			media_id: row.media_id,
			original_filename: row.original_filename ?? 'Untitled',
			src: null
		}));
}

/** Its failure costs the picker its options, not the page. */
async function myTopics(event: Parameters<PageServerLoad>[0]): Promise<Topic[]> {
	const body = await read<{ items?: Topic[] } | Topic[]>(event, '/api/topics');
	const data = body?.data;
	const items = Array.isArray(data) ? data : (data?.items ?? []);

	return items.filter((topic) => topic?.id);
}

export const load: PageServerLoad = async (event) => {
	const id = encodeURIComponent(event.params.id ?? '');
	const none = { project: null, denied: true, availableTopics: [] as Topic[], screenshots: [] };

	const body = await read<Project>(event, `/api/projects/${id}`);

	if (!body?.data?.id) return none;

	const [availableTopics, screenshots] = await Promise.all([
		myTopics(event),
		screenshotsFor(event, body.data.id)
	]);

	return { project: body.data, denied: false, availableTopics, screenshots };
};
