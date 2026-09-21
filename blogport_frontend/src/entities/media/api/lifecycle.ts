import { UNEXPECTED } from '$lib/shared/lib/api-failure';
import { handlingClass, type HandlingClass } from '$lib/shared/lib/error-class';

/**
 * The archive ladder for one image.
 *
 * §02's state table gives media the same three rungs as everything else:
 * `DELETE` soft, `POST …/restore`, `DELETE …/hard`. The console calls the first
 * "archive" rather than "delete" for the same reason a topic is "retired" — the
 * word has to match how reversible the act is, and only the third rung is not.
 *
 * In the entity because more than one feature removes an image: the post
 * editor's cover card, and the media library.
 */

type Fetch = typeof globalThis.fetch;

const mine: Fetch = (...args) => globalThis.fetch(...args);

export type LifecycleResult = { ok: true } | { ok: false; message: string; kind: HandlingClass };

async function call(
	path: string,
	method: 'POST' | 'DELETE',
	fetchFn: Fetch
): Promise<LifecycleResult> {
	let response: Response;

	try {
		response = await fetchFn(path, { method });
	} catch {
		return { ok: false, message: UNEXPECTED, kind: 'notOurs' };
	}

	if (!response.ok) {
		const body = (await response.json().catch(() => null)) as { error?: { code?: string } } | null;

		return { ok: false, message: UNEXPECTED, kind: handlingClass(body?.error?.code) };
	}

	return { ok: true };
}

const at = (mediaId: string) => `/api/media/${encodeURIComponent(mediaId)}`;

/** Soft delete. It drops out of every listing at once and can be brought back. */
export function archiveMedia(mediaId: string, fetchFn: Fetch = mine) {
	return call(at(mediaId), 'DELETE', fetchFn);
}

export function restoreMedia(mediaId: string, fetchFn: Fetch = mine) {
	return call(`${at(mediaId)}/restore`, 'POST', fetchFn);
}

/** The rung that does not come back. The stored object is the bucket's to reap. */
export function purgeMedia(mediaId: string, fetchFn: Fetch = mine) {
	return call(`${at(mediaId)}/hard`, 'DELETE', fetchFn);
}
