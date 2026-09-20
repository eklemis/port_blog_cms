import { UNEXPECTED } from '$lib/shared/lib/api-failure';
import { handlingClass, type HandlingClass } from '$lib/shared/lib/error-class';

/**
 * The editor's topic chips.
 *
 * §03 sets the rule these three share: "topics · combobox · own topics only —
 * **each chip is its own request; one failure doesn't roll back the others.**"
 * So every call here is about exactly one chip, and none of them knows the
 * others exist. A rail that half-succeeded is a rail that tells the truth.
 */

// The shape lives with the entity now that projects carry topics too.
export type { Topic } from '$lib/entities/topic';
// Creating a topic is the same call for a post and a project, so it lives with
// the entity; only attaching is target-specific.
export { createTopic } from '$lib/entities/topic';

export type TopicResult = { ok: true } | { ok: false; message: string; kind: HandlingClass };

type Fetch = typeof globalThis.fetch;

const mine: Fetch = (...args) => globalThis.fetch(...args);

async function readError(response: Response): Promise<{ message: string; kind: HandlingClass }> {
	const body = (await response.json().catch(() => null)) as { error?: { code?: string } } | null;
	return { message: UNEXPECTED, kind: handlingClass(body?.error?.code) };
}

async function chip(
	postId: string,
	topicId: string,
	method: 'POST' | 'DELETE',
	fetchFn: Fetch
): Promise<TopicResult> {
	let response: Response;

	try {
		response = await fetchFn(`/api/blog/${encodeURIComponent(postId)}/topics`, {
			method,
			headers: { 'content-type': 'application/json' },
			body: JSON.stringify({ topic_id: topicId })
		});
	} catch {
		return { ok: false, message: UNEXPECTED, kind: 'notOurs' };
	}

	if (!response.ok) return { ok: false, ...(await readError(response)) };

	return { ok: true };
}

export function attachTopic(postId: string, topicId: string, fetchFn: Fetch = mine) {
	return chip(postId, topicId, 'POST', fetchFn);
}

export function detachTopic(postId: string, topicId: string, fetchFn: Fetch = mine) {
	return chip(postId, topicId, 'DELETE', fetchFn);
}
