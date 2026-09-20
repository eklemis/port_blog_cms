import { UNEXPECTED } from '$lib/shared/lib/api-failure';
import { handlingClass } from '$lib/shared/lib/error-class';
import {
	beginUpload as start,
	type Done as MediaDone,
	type Failure as MediaFailure,
	type MediaState
} from '$lib/entities/media';

/**
 * Asking for a cover's upload URL is the general call with three values fixed.
 * `uploadBytes` is re-exported unchanged — the transfer does not care what it
 * is attached to.
 */
export { uploadBytes } from '$lib/entities/media';

export function beginUpload(
	{ postId, file, altText }: { postId: string; file: File; altText: string },
	fetchFn?: typeof globalThis.fetch
) {
	return start({ target: 'blog_post', targetId: postId, role: 'cover', file, altText }, fetchFn);
}

/**
 * The post's cover image.
 *
 * §03's insert-an-image flow, with `role: cover` rather than `inline`. Four
 * calls that are deliberately not one, because each fails differently and the
 * writer needs to know which:
 *
 * 1. `POST /api/media/upload-url` — the row is created here, in `pending`, and
 *    the id comes back *before the bytes exist*.
 * 2. `PUT` to the signed URL — **not this API**. The bytes go straight to
 *    storage and never pass through the backend, which is also why the browser
 *    is the only place the upload policy can really be checked.
 * 3. `GET /api/media/{id}` — poll. Variants are produced asynchronously by a
 *    separate service, so "uploaded" and "usable" are different moments.
 * 4. `DELETE /api/media/{id}` — soft delete; the row drops out of every read
 *    path immediately.
 */

type Fetch = typeof globalThis.fetch;

const mine: Fetch = (...args) => globalThis.fetch(...args);

export type { Done, Failure, Started } from '$lib/entities/media';

type Done = MediaDone;
type Failure = MediaFailure;

async function readError(response: Response): Promise<Failure> {
	const body = (await response.json().catch(() => null)) as { error?: { code?: string } } | null;

	return { ok: false, message: UNEXPECTED, kind: handlingClass(body?.error?.code) };
}

/**
 * Where the file is in processing.
 *
 * A row that has gone reads as `failed` rather than as still pending: 404 means
 * deleted or never there, and polling it forever would leave the rail spinning
 * over nothing.
 */
export async function readState(mediaId: string, fetchFn: Fetch = mine): Promise<MediaState> {
	let response: Response;

	try {
		response = await fetchFn(`/api/media/${encodeURIComponent(mediaId)}`);
	} catch {
		return 'failed';
	}

	if (!response.ok) return 'failed';

	const body = (await response.json().catch(() => null)) as {
		data?: { status?: MediaState };
	} | null;

	return body?.data?.status ?? 'failed';
}

/**
 * Correct the description after the fact.
 *
 * `PATCH /api/media/{id}` exists for this: "alt text, caption and position are
 * set at upload and were not editable, so a missing or wrong alt text was a
 * permanent accessibility defect." The picker is still the real moment, but a
 * card that offers no way back re-creates the defect the endpoint removed.
 *
 * Only `alt_text` is sent. PATCH changes what is present, so including a
 * `caption` key would clear a caption nobody asked to touch.
 */
export async function correctAltText(
	mediaId: string,
	altText: string,
	fetchFn: Fetch = mine
): Promise<Done> {
	let response: Response;

	try {
		response = await fetchFn(`/api/media/${encodeURIComponent(mediaId)}`, {
			method: 'PATCH',
			headers: { 'content-type': 'application/json' },
			body: JSON.stringify({ alt_text: altText })
		});
	} catch {
		return { ok: false, message: UNEXPECTED, kind: 'notOurs' };
	}

	if (!response.ok) return await readError(response);

	return { ok: true };
}

/** Take the cover off the post. Soft delete — the stored object is the bucket's. */
export async function removeCover(mediaId: string, fetchFn: Fetch = mine): Promise<Done> {
	let response: Response;

	try {
		response = await fetchFn(`/api/media/${encodeURIComponent(mediaId)}`, { method: 'DELETE' });
	} catch {
		return { ok: false, message: UNEXPECTED, kind: 'notOurs' };
	}

	if (!response.ok) return await readError(response);

	return { ok: true };
}
