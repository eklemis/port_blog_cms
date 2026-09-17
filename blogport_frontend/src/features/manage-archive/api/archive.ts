import { UNEXPECTED } from '$lib/shared/lib/api-failure';
import { handlingClass, type HandlingClass } from '$lib/shared/lib/error-class';
import type { components } from '$lib/shared/api/v1';

/**
 * The two ways out of the archive — §06's second and third rungs.
 *
 * Neither is optimistic. Restore could be, being reversible, but the row
 * leaving the list is the whole of its feedback and a row that leaves and
 * comes back reads as a glitch. Purge cannot be: optimism about an
 * irreversible action is just an inaccurate screen.
 */

export type ArchiveResult = { ok: true } | { ok: false; message: string; kind: HandlingClass };

/** Restored or purged elsewhere — another tab, another device. Stale, not broken. */
export const GONE = 'That post is no longer in the archive.';

async function call(
	path: string,
	method: 'POST' | 'DELETE',
	fetchFn: typeof globalThis.fetch
): Promise<ArchiveResult> {
	let response: Response;

	try {
		response = await fetchFn(path, { method });
	} catch {
		return { ok: false, message: UNEXPECTED, kind: 'notOurs' };
	}

	if (response.ok) return { ok: true };

	const body = (await response.json().catch(() => null)) as { error?: { code?: string } } | null;
	const kind = handlingClass(body?.error?.code);

	if (kind === 'notFound') return { ok: false, message: GONE, kind };

	return { ok: false, message: UNEXPECTED, kind: 'notOurs' };
}

const at = (id: string, action: 'restore' | 'hard') =>
	`/api/blog/${encodeURIComponent(id)}/${action}`;

/**
 * §06's first rung. A DELETE on the wire, an archive in the interface: the row
 * survives and `restorePost` brings it back, which is what the undo toast does.
 */
export function archivePost(
	id: string,
	fetchFn: typeof globalThis.fetch = (...args) => globalThis.fetch(...args)
) {
	return call(`/api/blog/${encodeURIComponent(id)}`, 'DELETE', fetchFn);
}

export function restorePost(
	id: string,
	fetchFn: typeof globalThis.fetch = (...args) => globalThis.fetch(...args)
) {
	return call(at(id, 'restore'), 'POST', fetchFn);
}

export function purgePost(
	id: string,
	fetchFn: typeof globalThis.fetch = (...args) => globalThis.fetch(...args)
) {
	return call(at(id, 'hard'), 'DELETE', fetchFn);
}

/**
 * The operations this call can send, taken from the spec rather than typed out.
 *
 * It was written by hand from the prose, which listed five ops where the enum
 * had six — so `unpublish` was missing from the type rather than merely unused,
 * and nothing could have caught that. Derived, it cannot fall behind again.
 *
 * The topic operations are excluded because they carry a `topic_id` this
 * signature has nowhere to put; a caller that needs them needs a different
 * call, not a wider string.
 */
export type BulkOp = Exclude<components['schemas']['BlogBulkOp'], { topic_id: string }>['op'];

export type BulkResult =
	| {
			ok: true;
			succeeded: string[];
			/** Each failed id with a sentence of its own, classified by its own code. */
			failed: Record<string, string>;
	  }
	| { ok: false; message: string; kind: HandlingClass };

/**
 * One operation over a selection.
 *
 * "success: true means the batch ran, not that every item did" — so a 200 is
 * not a yes. Failures come back per item with the same codes a single call
 * would have, and each keeps its own sentence.
 */
export async function bulkPosts(
	op: BulkOp,
	ids: string[],
	fetchFn: typeof globalThis.fetch = (...args) => globalThis.fetch(...args)
): Promise<BulkResult> {
	let response: Response;

	try {
		response = await fetchFn('/api/blog/bulk', {
			method: 'POST',
			headers: { 'content-type': 'application/json' },
			body: JSON.stringify({ op, ids })
		});
	} catch {
		return { ok: false, message: UNEXPECTED, kind: 'notOurs' };
	}

	const body = (await response.json().catch(() => null)) as {
		succeeded?: string[];
		failed?: { id: string; code: string }[];
		error?: { code?: string };
	} | null;

	if (!response.ok || !body) {
		return { ok: false, message: UNEXPECTED, kind: handlingClass(body?.error?.code) };
	}

	const failed: Record<string, string> = {};
	for (const item of body.failed ?? []) {
		failed[item.id] = handlingClass(item.code) === 'notFound' ? GONE : UNEXPECTED;
	}

	return { ok: true, succeeded: body.succeeded ?? [], failed };
}
