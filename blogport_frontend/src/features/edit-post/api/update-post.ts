import { UNEXPECTED } from '$lib/shared/lib/api-failure';
import type { HandlingClass } from '$lib/shared/lib/error-class';
import type { components } from '$lib/shared/api/v1';

/**
 * Saving an edit — the call the autosave loop makes.
 *
 * PATCH only, and only the keys that moved: the API changes what is present and
 * leaves out what is not, so the editor never sends an object it did not load
 * first. It never rejects; the loop reads the result and decides whether to
 * retry, and a rejection there would be a retry loop with no state.
 */

export type PostChanges = Pick<
	components['schemas']['PatchBlogPostRequest'],
	'title' | 'slug' | 'content' | 'excerpt' | 'published_at'
>;

/** Ours, not "Slug already exists" — that sentence is written for a log. */
export const SLUG_TAKEN = 'That address is already in use.';

/** J4: someone else's post, or one that is gone. Either way, not a field. */
export const NO_ACCESS = "You don't have access to this post.";

export type EditField = 'title' | 'slug' | 'content';

export type PatchResult =
	| { ok: true }
	| { ok: false; field: EditField | null; message: string; kind: HandlingClass };

const FIELD_OF: Record<string, EditField> = {
	INVALID_TITLE: 'title',
	INVALID_SLUG: 'slug',
	INVALID_CONTENT: 'content'
};

function failed(message: string, field: EditField | null, kind: HandlingClass): PatchResult {
	return { ok: false, field, message, kind };
}

export async function patchPost(
	id: string,
	changes: PostChanges,
	fetchFn: typeof globalThis.fetch = (...args) => globalThis.fetch(...args)
): Promise<PatchResult> {
	let response: Response;

	try {
		response = await fetchFn(`/api/blog/${encodeURIComponent(id)}`, {
			method: 'PATCH',
			headers: { 'content-type': 'application/json' },
			body: JSON.stringify(changes)
		});
	} catch {
		return failed(UNEXPECTED, null, 'notOurs');
	}

	if (response.ok) return { ok: true };

	const body = (await response.json().catch(() => null)) as {
		error?: { code?: string; message?: string };
	} | null;

	const code = body?.error?.code ?? '';

	if (code === 'SLUG_ALREADY_EXISTS') return failed(SLUG_TAKEN, 'slug', 'collision');

	// J4 names POST_UNAUTHORIZED; the API reports someone else's post as not
	// found instead, so both arrive here. Neither is a field to correct.
	if (code === 'POST_NOT_FOUND' || code === 'POST_UNAUTHORIZED' || response.status === 404) {
		return failed(NO_ACCESS, null, 'gate');
	}

	const field = FIELD_OF[code];
	if (field) return failed(body?.error?.message ?? UNEXPECTED, field, 'field');

	return failed(UNEXPECTED, null, 'notOurs');
}
