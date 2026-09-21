import type { components } from '$lib/shared/api/v1';

/**
 * An image attached to something, and what its processing state means.
 *
 * The lifecycle is the API's, not ours: "A row is created before the bytes
 * arrive, so a media item existing does not mean the file does." That is why
 * the card has states at all — an upload is not an event, it is a period.
 */

export type MediaState = components['schemas']['MediaState'];
export type MediaRole = components['schemas']['MediaRole'];
export type MediaSize = components['schemas']['MediaSize'];

/**
 * The part of `MediaItem` anything displaying an attachment needs. Narrower
 * than the wire type on purpose: a component that takes `original_filename` is
 * a component that will eventually show it.
 */
export type Attachment = {
	media_id: string;
	attachment_target_id: string;
	role: MediaRole;
	status: MediaState;
	alt_text: string;
};

/**
 * This post's cover, out of every blog-post attachment the caller owns.
 *
 * The listing can be asked for one target and one role now, so callers get a
 * short list rather than the author's whole library. This still narrows it:
 * asking for one post's cover should return one row, and code that assumes the
 * server filtered perfectly is code that breaks the day it does not.
 */
export function coverOf(rows: readonly Attachment[], postId: string): Attachment | null {
	return rows.find((row) => row.attachment_target_id === postId && row.role === 'cover') ?? null;
}

type Pill = { tone: 'inflight' | 'danger'; label: string };

/**
 * What to say while an image is not yet an image — and nothing once it is.
 *
 * `ready` has no pill because the picture has arrived and is its own status. A
 * label over a visible image is a caption saying "this is a picture".
 */
export function pillFor(state: MediaState): Pill | null {
	if (state === 'failed') return { tone: 'danger', label: 'Failed' };
	if (state === 'ready') return null;

	return { tone: 'inflight', label: state === 'pending' ? 'Pending' : 'Processing' };
}
