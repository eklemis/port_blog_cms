import { UNEXPECTED } from '$lib/shared/lib/api-failure';
import type { HandlingClass } from '$lib/shared/lib/error-class';

/**
 * Creating a project, through the console's proxy.
 *
 * **Creating one publishes it.** §02: "Projects have no `published_at`.
 * Creating one publishes it." There is no draft to come back to and tidy up in
 * private, which is why the form says so before the button rather than after.
 *
 * Deliberately small, for the same reason the post's create form is: media
 * attaches to a `target_id` and topics attach by id, so both need the project
 * to exist first. What is here is what a project needs in order to be a
 * project — a name, an address and a sentence saying what it is. Everything
 * else is the editor's, which is where this leads.
 */

export const CREATE_ROUTE = '/api/projects';
export const SLUG_ROUTE = '/api/projects/slug-available';

/**
 * Ours, not the backend's "Slug already exists" — that sentence is written for
 * whoever is reading a log. This one is read by someone who has just typed an
 * address, and the suggestion beside it is what they do next.
 */
export const SLUG_TAKEN = 'That address is already in use.';

export type CreateField = 'title' | 'slug' | 'description';

export type CreateResult =
	| { ok: true; id: string }
	| { ok: false; field: CreateField | null; message: string; kind: HandlingClass };

const FIELD_OF: Record<string, CreateField> = {
	INVALID_TITLE: 'title',
	EMPTY_TITLE: 'title',
	TITLE_TOO_LONG: 'title',
	INVALID_SLUG: 'slug'
};

function failed(
	message: string,
	{ field = null, kind = 'notOurs' }: { field?: CreateField | null; kind?: HandlingClass } = {}
): CreateResult {
	return { ok: false, field, message, kind };
}

export async function createProject(
	project: { title: string; slug: string; description: string; tech_stack?: string[] },
	fetchFn: typeof globalThis.fetch = (...args) => globalThis.fetch(...args)
): Promise<CreateResult> {
	let response: Response;

	try {
		response = await fetchFn(CREATE_ROUTE, {
			method: 'POST',
			headers: { 'content-type': 'application/json' },
			body: JSON.stringify(project)
		});
	} catch {
		return failed(UNEXPECTED);
	}

	const body = (await response.json().catch(() => null)) as {
		data?: { id?: string };
		error?: { code?: string; message?: string };
	} | null;

	if (response.ok) {
		const id = body?.data?.id;

		// Without an id there is nowhere to send the person next, so this is a
		// failure rather than a project that quietly exists.
		if (!id) return failed(UNEXPECTED);

		return { ok: true, id };
	}

	const code = body?.error?.code;

	// §02: "SLUG_ALREADY_EXISTS · 409. Same handling as posts: suggest, don't
	// discard."
	if (code === 'SLUG_ALREADY_EXISTS') {
		return failed(SLUG_TAKEN, { field: 'slug', kind: 'collision' });
	}

	const field = code ? FIELD_OF[code] : undefined;

	if (field) {
		// The server's message names the rule it refused, and the server is the
		// authority on what it will accept.
		return failed(body?.error?.message ?? UNEXPECTED, { field, kind: 'field' });
	}

	return failed(UNEXPECTED);
}

export type SlugCheck = { available: boolean; suggestion: string | null };

/**
 * Ask whether an address is free, before the create call has to say so.
 *
 * It matters more here than on a post: `PatchProjectRequest` carries no slug,
 * so a project's address cannot be corrected afterwards. This is the one
 * moment it can be got right.
 *
 * Still a courtesy rather than a gate. A check that could not be made reports
 * "available", because reporting "taken" would stop someone submitting an
 * address that was free all along.
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
