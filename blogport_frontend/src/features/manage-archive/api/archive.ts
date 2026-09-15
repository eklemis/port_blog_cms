import { UNEXPECTED } from '$lib/shared/lib/api-failure';
import { handlingClass, type HandlingClass } from '$lib/shared/lib/error-class';

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
