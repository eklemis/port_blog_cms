import { UNEXPECTED } from '$lib/shared/lib/api-failure';
import { handlingClass, type HandlingClass } from '$lib/shared/lib/error-class';

/**
 * Saving a project, and putting topics on it.
 *
 * §02: "there is no full-replace PUT, so the editor is field-level throughout
 * and never sends an object it did not load first." So every call here carries
 * only what moved.
 *
 * **`slug` is not among the fields.** `CreateProjectRequest` takes one and
 * `PatchProjectRequest` does not, so a project's address is fixed at creation.
 * The editor renders it as a fact rather than a field; see the editor's own
 * note, and §04: "Read-only facts render as text, not disabled inputs."
 */

type Fetch = typeof globalThis.fetch;

const mine: Fetch = (...args) => globalThis.fetch(...args);

/** What can actually be changed. `null` clears; an omitted key is left alone. */
export type ProjectChanges = {
	title?: string;
	description?: string | null;
	repo_url?: string | null;
	live_demo_url?: string | null;
	tech_stack?: string[];
};

export type SaveResult = { ok: true } | { ok: false; message: string; kind: HandlingClass };

async function refusal(response: Response): Promise<SaveResult> {
	const body = (await response.json().catch(() => null)) as { error?: { code?: string } } | null;

	return { ok: false, message: UNEXPECTED, kind: handlingClass(body?.error?.code) };
}

export async function patchProject(
	projectId: string,
	changes: ProjectChanges,
	fetchFn: Fetch = mine
): Promise<SaveResult> {
	let response: Response;

	try {
		response = await fetchFn(`/api/projects/${encodeURIComponent(projectId)}`, {
			method: 'PATCH',
			headers: { 'content-type': 'application/json' },
			body: JSON.stringify(changes)
		});
	} catch {
		return { ok: false, message: UNEXPECTED, kind: 'notOurs' };
	}

	if (!response.ok) return await refusal(response);

	return { ok: true };
}

/**
 * One chip, one request — §03's rule for the post's topics, and the same here:
 * "each chip is its own request; one failure doesn't roll back the others."
 */
async function chip(
	projectId: string,
	topicId: string,
	method: 'POST' | 'DELETE',
	fetchFn: Fetch
): Promise<SaveResult> {
	let response: Response;

	try {
		response = await fetchFn(`/api/projects/${encodeURIComponent(projectId)}/topics`, {
			method,
			headers: { 'content-type': 'application/json' },
			body: JSON.stringify({ topic_id: topicId })
		});
	} catch {
		return { ok: false, message: UNEXPECTED, kind: 'notOurs' };
	}

	if (!response.ok) return await refusal(response);

	return { ok: true };
}

export function attachTopic(projectId: string, topicId: string, fetchFn: Fetch = mine) {
	return chip(projectId, topicId, 'POST', fetchFn);
}

export function detachTopic(projectId: string, topicId: string, fetchFn: Fetch = mine) {
	return chip(projectId, topicId, 'DELETE', fetchFn);
}
