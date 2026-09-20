import { UNEXPECTED } from '$lib/shared/lib/api-failure';
import { handlingClass, type HandlingClass } from '$lib/shared/lib/error-class';
import type { TopicUsage } from '$lib/entities/topic';

/**
 * Managing the shared vocabulary — rename, retire, and count first.
 *
 * **Renaming is possible now, and §02 says it is not.** Its journey reads
 * "Retire, don't rename — only soft delete exists... No update endpoint. A typo
 * is unfixable." `PATCH /api/topics/{id}` has since shipped, and its own
 * description says what it replaced: "a typo in a title was permanent and
 * visible on every tagged post and project. The workaround was
 * create-retag-retire, by hand." The topic keeps its id, so nothing needs
 * retagging. Raised with the designer rather than resolved quietly.
 *
 * Retiring is still a soft delete, and "Retire" is still the right word for it.
 */

type Fetch = typeof globalThis.fetch;

const mine: Fetch = (...args) => globalThis.fetch(...args);

export type TopicResult = { ok: true } | { ok: false; message: string; kind: HandlingClass };

function path(topicId: string): string {
	return `/api/topics/${encodeURIComponent(topicId)}`;
}

async function refusal(response: Response, title?: string): Promise<TopicResult> {
	const body = (await response.json().catch(() => null)) as { error?: { code?: string } } | null;
	const code = body?.error?.code;

	// §02: "TOPIC_ALREADY_EXISTS · 409. In the inline creator this is a
	// near-miss, not a failure." It is the same here — the name exists, and the
	// person wanted that name.
	if (code === 'TOPIC_ALREADY_EXISTS' && title) {
		return { ok: false, message: `You already have a topic called ${title}.`, kind: 'collision' };
	}

	return { ok: false, message: UNEXPECTED, kind: handlingClass(code) };
}

export async function renameTopic(
	topicId: string,
	title: string,
	fetchFn: Fetch = mine
): Promise<TopicResult> {
	let response: Response;

	try {
		response = await fetchFn(path(topicId), {
			method: 'PATCH',
			headers: { 'content-type': 'application/json' },
			// Only the title. A description sent as null would clear one nobody
			// asked to touch.
			body: JSON.stringify({ title })
		});
	} catch {
		return { ok: false, message: UNEXPECTED, kind: 'notOurs' };
	}

	if (!response.ok) return await refusal(response, title);

	return { ok: true };
}

/** Soft delete. The row stays, so existing references keep resolving. */
export async function retireTopic(topicId: string, fetchFn: Fetch = mine): Promise<TopicResult> {
	let response: Response;

	try {
		response = await fetchFn(path(topicId), { method: 'DELETE' });
	} catch {
		return { ok: false, message: UNEXPECTED, kind: 'notOurs' };
	}

	if (!response.ok) return await refusal(response);

	return { ok: true };
}

/**
 * How many things a topic is on.
 *
 * `null` when it could not be counted — never zero. Zero reads as "nothing is
 * using it" and invites a confident retire, which is the mistake the endpoint
 * was added to prevent.
 */
export async function topicUsage(
	topicId: string,
	fetchFn: Fetch = mine
): Promise<TopicUsage | null> {
	try {
		const response = await fetchFn(`${path(topicId)}/usage`);
		if (!response.ok) return null;

		const body = (await response.json()) as { data?: Partial<TopicUsage> };
		const data = body.data;

		if (typeof data?.posts !== 'number' || typeof data?.projects !== 'number') return null;

		return { posts: data.posts, projects: data.projects };
	} catch {
		return null;
	}
}
