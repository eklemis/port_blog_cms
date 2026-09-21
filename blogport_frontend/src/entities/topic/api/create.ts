import { UNEXPECTED } from '$lib/shared/lib/api-failure';
import { handlingClass, type HandlingClass } from '$lib/shared/lib/error-class';
import type { Topic } from '../model/topic';

/**
 * `POST /api/topics` — the picker's inline create.
 *
 * Creating a topic is not target-specific: the same call serves a post's rail
 * and a project's. Attaching is, so it stays with whichever feature owns the
 * target.
 *
 * §03: "Create topic (inline) · title, description · Create & attach." This is
 * only the create. A topic that exists but is not yet attached is a truthful
 * intermediate state, and collapsing the two would hide which half failed.
 */

type Fetch = typeof globalThis.fetch;

const mine: Fetch = (...args) => globalThis.fetch(...args);

export type CreatedTopic =
	| { ok: true; topic: Topic }
	| { ok: false; message: string; kind: HandlingClass };

export async function createTopic(
	title: string,
	fetchFn: Fetch = mine,
	description?: string
): Promise<CreatedTopic> {
	let response: Response;

	try {
		response = await fetchFn('/api/topics', {
			method: 'POST',
			headers: { 'content-type': 'application/json' },
			// The description is omitted rather than sent empty: the editor rail's
			// inline creator has nowhere to type one, and an empty string is a
			// value the row would then carry.
			body: JSON.stringify(
				description?.trim() ? { title, description: description.trim() } : { title }
			)
		});
	} catch {
		return { ok: false, message: UNEXPECTED, kind: 'notOurs' };
	}

	if (!response.ok) {
		const body = (await response.json().catch(() => null)) as { error?: { code?: string } } | null;
		const code = body?.error?.code;

		// §02: "In the inline creator this is a near-miss, not a failure: 'You
		// already have a topic called Rust'."
		if (code === 'TOPIC_ALREADY_EXISTS') {
			return { ok: false, message: `You already have a topic called ${title}.`, kind: 'collision' };
		}

		return { ok: false, message: UNEXPECTED, kind: handlingClass(code) };
	}

	const body = (await response.json().catch(() => null)) as { data?: Partial<Topic> } | null;
	const topic = body?.data;

	// A topic with no id cannot be attached, so it is not one we can use.
	if (!topic?.id) return { ok: false, message: UNEXPECTED, kind: 'notOurs' };

	return { ok: true, topic: { id: topic.id, title: topic.title ?? title } };
}
