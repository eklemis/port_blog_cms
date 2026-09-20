import { UNEXPECTED } from '$lib/shared/lib/api-failure';
import { handlingClass, type HandlingClass } from '$lib/shared/lib/error-class';

/**
 * `PATCH /api/media/{id}` — correcting an attachment's metadata.
 *
 * Lives in the entity because two features need it and slices in the same layer
 * may not import each other: the post editor corrects a cover's alt text, the
 * project editor moves a screenshot. Writing it twice would be two places for
 * the same request shape to drift.
 *
 * The endpoint exists because "alt text, caption and position are set at upload
 * and were not editable, so a missing or wrong alt text was a permanent
 * accessibility defect and a gallery could not be reordered without
 * re-uploading every image."
 */

type Fetch = typeof globalThis.fetch;

const mine: Fetch = (...args) => globalThis.fetch(...args);

/** What the endpoint will change. Omitted keys are left alone; `null` clears. */
export type MediaChanges = {
	alt_text?: string | null;
	caption?: string | null;
	position?: number | null;
};

export type PatchResult = { ok: true } | { ok: false; message: string; kind: HandlingClass };

export async function patchMedia(
	mediaId: string,
	changes: MediaChanges,
	fetchFn: Fetch = mine
): Promise<PatchResult> {
	let response: Response;

	try {
		response = await fetchFn(`/api/media/${encodeURIComponent(mediaId)}`, {
			method: 'PATCH',
			headers: { 'content-type': 'application/json' },
			// Exactly what the caller asked to change: a key sent as null clears
			// the value, so an unasked-for key is a silent deletion.
			body: JSON.stringify(changes)
		});
	} catch {
		return { ok: false, message: UNEXPECTED, kind: 'notOurs' };
	}

	if (!response.ok) {
		const body = (await response.json().catch(() => null)) as { error?: { code?: string } } | null;

		return { ok: false, message: UNEXPECTED, kind: handlingClass(body?.error?.code) };
	}

	return { ok: true };
}
