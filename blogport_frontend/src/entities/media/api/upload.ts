import { UNEXPECTED } from '$lib/shared/lib/api-failure';
import { handlingClass, type HandlingClass } from '$lib/shared/lib/error-class';
import type { components } from '$lib/shared/api/v1';
import type { MediaRole } from '../model/media';

/**
 * Starting an upload, for whatever it is attached to.
 *
 * Two features need this — a post's cover and a project's screenshots — and
 * slices in the same layer may not import each other, so it lives below both.
 * What differs between the two is three values: target, id and role.
 *
 * The shape of the flow is §03's: ask for a URL, then PUT the bytes somewhere
 * that is not this API. The id comes back *before the bytes exist*, which is
 * why polling is a separate concern and lives with whoever is watching.
 *
 * **The wire forms are lowercase and snake_case.** ADR 0008 collapsed the
 * `screenshoot` rename and fixed the serde format in the same step:
 * "`POST /api/media/upload-url` now takes `"screenshot"` and `"blog_post"`
 * where it took `"Screenshoot"` and `"BlogPost"`. Any client sending the
 * capitalized forms breaks."
 */

type Fetch = typeof globalThis.fetch;

const mine: Fetch = (...args) => globalThis.fetch(...args);

export type AttachmentTarget = components['schemas']['AttachmentTarget'];

export type Failure = { ok: false; message: string; kind: HandlingClass };

export type Started = { ok: true; mediaId: string; uploadUrl: string } | Failure;

export type Done = { ok: true } | Failure;

export type UploadRequest = {
	target: AttachmentTarget;
	targetId: string;
	role: MediaRole;
	file: File;
	/**
	 * Required by §03's plan, not by the API: "Blocks upload. Editable
	 * afterwards via PATCH, but treat the picker as the real moment."
	 */
	altText: string;
	/** Where it lands in its role's order. A cover has no order to land in. */
	position?: number;
};

/**
 * Ask for somewhere to put the file.
 *
 * Everything the upload policy is written in is declared here — size, type,
 * name — because the service never sees the bytes and cannot check them.
 */
export async function beginUpload(
	{ target, targetId, role, file, altText, position }: UploadRequest,
	fetchFn: Fetch = mine
): Promise<Started> {
	let response: Response;

	try {
		response = await fetchFn('/api/media/upload-url', {
			method: 'POST',
			headers: { 'content-type': 'application/json' },
			body: JSON.stringify({
				attachment_target: target,
				attachment_target_id: targetId,
				role,
				file_name: file.name,
				file_size_bytes: file.size,
				mime_type: file.type,
				alt_text: altText,
				// Omitted rather than sent as null: the server has a default, and a
				// caller with no opinion should not overwrite it.
				...(position === undefined ? {} : { position })
			})
		});
	} catch {
		return { ok: false, message: UNEXPECTED, kind: 'notOurs' };
	}

	if (!response.ok) {
		const body = (await response.json().catch(() => null)) as { error?: { code?: string } } | null;

		return { ok: false, message: UNEXPECTED, kind: handlingClass(body?.error?.code) };
	}

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
 * with a progress indicator on it.
 *
 * The bucket needs a CORS policy for this to work at all: the browser sends an
 * `OPTIONS` preflight first, because the `PUT` carries a content type. See
 * `backend_actix/infra/README.md`.
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
