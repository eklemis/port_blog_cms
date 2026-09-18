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

export type Topic = { id: string; title: string };

export type TopicResult = { ok: true } | { ok: false; message: string; kind: HandlingClass };

export type CreatedTopic =
	| { ok: true; topic: Topic }
	| { ok: false; message: string; kind: HandlingClass };

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

/**
 * Inline create — §03: "Create topic (inline) · title, description · Create &
 * attach · Chip attaches, popover closes."
 *
 * Creating and attaching are two requests, and this is only the first: a topic
 * that exists but is not yet on the post is a truthful intermediate state, and
 * collapsing them would hide which half failed.
 */
export async function createTopic(title: string, fetchFn: Fetch = mine): Promise<CreatedTopic> {
	let response: Response;

	try {
		response = await fetchFn('/api/topics', {
			method: 'POST',
			headers: { 'content-type': 'application/json' },
			body: JSON.stringify({ title })
		});
	} catch {
		return { ok: false, message: UNEXPECTED, kind: 'notOurs' };
	}

	if (!response.ok) return { ok: false, ...(await readError(response)) };

	const body = (await response.json().catch(() => null)) as { data?: Partial<Topic> } | null;
	const topic = body?.data;

	// A topic with no id cannot be attached, so it is not a topic we can use.
	if (!topic?.id) return { ok: false, message: UNEXPECTED, kind: 'notOurs' };

	return { ok: true, topic: { id: topic.id, title: topic.title ?? title } };
}
