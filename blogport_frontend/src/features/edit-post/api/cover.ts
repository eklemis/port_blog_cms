import { UNEXPECTED } from '$lib/shared/lib/api-failure';
import { handlingClass, type HandlingClass } from '$lib/shared/lib/error-class';
import type { MediaState } from '$lib/entities/media';

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

export type Failure = { ok: false; message: string; kind: HandlingClass };

export type Started = { ok: true; mediaId: string; uploadUrl: string } | Failure;

export type Done = { ok: true } | Failure;

async function readError(response: Response): Promise<Failure> {
	const body = (await response.json().catch(() => null)) as { error?: { code?: string } } | null;

	return { ok: false, message: UNEXPECTED, kind: handlingClass(body?.error?.code) };
}

/**
 * Ask for somewhere to put the file.
 *
 * Everything the upload policy is written in is declared here — size, type,
 * name — because the service never sees the bytes and cannot check them. Alt
 * text goes up with the request rather than afterwards: §03 treats the picker
 * as the real moment, since nobody comes back to it.
 */
export async function beginUpload(
	{ postId, file, altText }: { postId: string; file: File; altText: string },
	fetchFn: Fetch = mine
): Promise<Started> {
	let response: Response;

	try {
		response = await fetchFn('/api/media/upload-url', {
			method: 'POST',
			headers: { 'content-type': 'application/json' },
			body: JSON.stringify({
				attachment_target: 'blog_post',
				attachment_target_id: postId,
				role: 'cover',
				file_name: file.name,
				file_size_bytes: file.size,
				mime_type: file.type,
				alt_text: altText
			})
		});
	} catch {
		return { ok: false, message: UNEXPECTED, kind: 'notOurs' };
	}

	if (!response.ok) return await readError(response);

	const body = (await response.json().catch(() => null)) as {
		data?: { media_id?: string; upload_url?: string };
	} | null;

	const mediaId = body?.data?.media_id;
	const uploadUrl = body?.data?.upload_url;

	// Without both there is nothing to PUT to and nothing to poll, so this is a
	// failure rather than an upload that has quietly started.
	if (!mediaId || !uploadUrl) return { ok: false, message: UNEXPECTED, kind: 'notOurs' };

	return { ok: true, mediaId, uploadUrl };
}

/**
 * Send the bytes to storage.
 *
 * The one call in this codebase that is not `fetch`. §03 asks for determinate
 * progress here — "the only honest bar" — and `fetch` cannot report how much of
 * a request body has gone out. An animated bar over a real transfer is a lie
 * with a progress indicator on it, so this uses `XMLHttpRequest` and measures.
 */
export function uploadBytes(
	url: string,
	file: File,
	{
		onprogress,
		open = () => new XMLHttpRequest()
	}: { onprogress?: (fraction: number) => void; open?: () => XMLHttpRequest } = {}
): Promise<Done> {
	return new Promise((resolve) => {
		const request = open();

		request.upload.onprogress = (event) => {
			// A length that is not computable is not a number to draw a bar from.
			if (!event.lengthComputable || !event.total) return;

			onprogress?.(event.loaded / event.total);
		};

		request.onload = () => {
			if (request.status >= 200 && request.status < 300) return resolve({ ok: true });

			resolve({ ok: false, message: UNEXPECTED, kind: 'notOurs' });
		};

		request.onerror = () => resolve({ ok: false, message: UNEXPECTED, kind: 'notOurs' });

		request.open('PUT', url);
		request.setRequestHeader('content-type', file.type);
		request.send(file);
	});
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
