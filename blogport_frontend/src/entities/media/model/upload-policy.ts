/**
 * What this product will accept as an image, checked before anything uploads.
 *
 * §03's media upload table: "≤ 5 MB, ≤ 6000px/side, JPEG/PNG/WebP — all three
 * checked in the browser first." The limits themselves come from the server's
 * upload policy, quoted on `POST /api/media/upload-url`.
 *
 * **This is not a duplicate of a server check.** The bytes never reach the API:
 * they go straight to a signed GCS URL, and `InitUploadRequest.mime_type` says
 * so in as many words — "as declared by the client. Never checked against the
 * bytes, which never reach this service." For the pixel dimensions the browser
 * is the only place the check can happen at all, because only the browser ever
 * decodes the file.
 *
 * A rejection carries the same code the API would have used, so one sentence
 * table covers both paths and `error-class` files them all under
 * `fileRejected`: "On the drop zone, before anything uploads. The form is
 * untouched."
 */

/** 5 MB, as the upload policy states it. */
export const MAX_BYTES = 5 * 1024 * 1024;

/** 6000 pixels on the longest side. */
export const MAX_EDGE = 6000;

/** The three the policy names, in the order it names them. */
export const ACCEPTED_TYPES = ['image/jpeg', 'image/png', 'image/webp'] as const;

export type RejectionCode = 'FILE_TOO_LARGE' | 'INVALID_MIME_TYPE' | 'INVALID_DIMENSIONS';

export type Rejection = {
	code: RejectionCode;
	/** A sentence for the drop zone. States the rule and the measurement. */
	message: string;
};

/** "8.2 MB" — one decimal, because "8.24 MB" is precision nobody asked for. */
function megabytes(bytes: number): string {
	return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

/**
 * What can be known from the file handle alone: its declared size and type.
 *
 * Size is checked first on purpose. A 9 MB GIF breaks two rules, and a drop
 * zone that answers with a list of faults is harder to act on than one that
 * names the first.
 */
export function checkDeclared(file: {
	name: string;
	size: number;
	type: string;
}): Rejection | null {
	if (file.size > MAX_BYTES) {
		return {
			code: 'FILE_TOO_LARGE',
			message: `Images must be 5 MB or smaller. That one is ${megabytes(file.size)}.`
		};
	}

	if (!(ACCEPTED_TYPES as readonly string[]).includes(file.type)) {
		// By type rather than by extension: the extension is the writer's to get
		// wrong, and the policy is written in types.
		return { code: 'INVALID_MIME_TYPE', message: 'Images must be a JPEG, PNG or WebP.' };
	}

	return null;
}

/** What can only be known once the browser has decoded the image. */
export function checkDimensions(width: number, height: number): Rejection | null {
	const longest = Math.max(width, height);

	if (longest > MAX_EDGE) {
		return {
			code: 'INVALID_DIMENSIONS',
			message: `Images must be 6000px or smaller on each side. That one is ${longest}px.`
		};
	}

	return null;
}
