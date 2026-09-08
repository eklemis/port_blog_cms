import { UNEXPECTED, rateLimited, retryAfterSeconds } from '$lib/shared/lib/api-failure';
import type { HandlingClass } from '$lib/shared/lib/error-class';

/**
 * Creating the draft — J4 step one, through the console's proxy.
 *
 * Deliberately small: media attaches to a `target_id`, so the post has to exist
 * before it can have a cover image, and everything after the first save belongs
 * to the editor.
 */

export const CREATE_ROUTE = '/api/blog';
export const SLUG_ROUTE = '/api/blog/slug-available';

/**
 * Ours, not the backend's "Slug already exists" — that sentence is written for
 * whoever is reading a log. This one is read by someone who has just typed an
 * address, and the suggestion beside it is what they actually do next.
 */
export const SLUG_TAKEN = 'That address is already in use.';

/** Which field a failure belongs under, when it belongs under one at all. */
export type CreateField = 'title' | 'slug' | 'content';

export type CreateResult =
	| { ok: true; id: string }
	| {
			ok: false;
			field: CreateField | null;
			message: string;
			retryAfterSeconds: number | null;
			kind: HandlingClass;
	  };

const FIELD_OF: Record<string, CreateField> = {
	INVALID_TITLE: 'title',
	INVALID_SLUG: 'slug',
	INVALID_CONTENT: 'content'
};

/** `notOurs` by default, the same fallback the mapping itself takes. */
function failed(
	message: string,
	{
		field = null,
		seconds = null,
		kind = 'notOurs'
	}: { field?: CreateField | null; seconds?: number | null; kind?: HandlingClass } = {}
): CreateResult {
	return { ok: false, field, message, retryAfterSeconds: seconds, kind };
}

export async function createPost(
	draft: { title: string; slug: string; content: string },
	fetchFn: typeof globalThis.fetch = (...args) => globalThis.fetch(...args)
): Promise<CreateResult> {
	let response: Response;

	try {
		response = await fetchFn(CREATE_ROUTE, {
			method: 'POST',
			headers: { 'content-type': 'application/json' },
			// No `published_at` at all. Omitting it is what makes this a draft;
			// sending null is a different statement, and the API reads it as
			// clearing a date rather than never having set one.
			body: JSON.stringify(draft)
		});
	} catch {
		return failed(UNEXPECTED);
	}

	const body = (await response.json().catch(() => null)) as {
		id?: string;
		error?: { code?: string; message?: string };
	} | null;

	if (response.ok && body?.id) return { ok: true, id: body.id };

	const code = body?.error?.code ?? '';

	if (code === 'SLUG_ALREADY_EXISTS') {
		// J4: mark the field and never lose the draft body to a collision.
		return failed(SLUG_TAKEN, { field: 'slug', kind: 'collision' });
	}

	if (code === 'RATE_LIMITED') {
		const seconds = retryAfterSeconds(response);
		return failed(rateLimited(seconds), { seconds, kind: 'wait' });
	}

	const field = FIELD_OF[code];
	if (field) {
		// The server's message names the rule it refused. Ours mirrors the same
		// rules, so arriving here means they disagreed — and the server is the
		// authority on what it will accept.
		return failed(body?.error?.message ?? UNEXPECTED, { field, kind: 'field' });
	}

	return failed(UNEXPECTED);
}

export type SlugCheck = { available: boolean; suggestion: string | null };

/**
 * Ask whether an address is free, before the create call has to say so.
 *
 * A courtesy, not a gate. The real answer is `SLUG_ALREADY_EXISTS` on create,
 * so a check that could not be made reports "available" — reporting "taken"
 * would stop someone submitting an address that was free all along.
 */
export async function checkSlug(
	slug: string,
	fetchFn: typeof globalThis.fetch = (...args) => globalThis.fetch(...args)
): Promise<SlugCheck> {
	const free: SlugCheck = { available: true, suggestion: null };

	try {
		const query = new URLSearchParams({ slug });
		const response = await fetchFn(`${SLUG_ROUTE}?${query}`);
		if (!response.ok) return free;

		const body = (await response.json()) as Partial<SlugCheck>;
		if (typeof body.available !== 'boolean') return free;

		return { available: body.available, suggestion: body.suggestion ?? null };
	} catch {
		return free;
	}
}
