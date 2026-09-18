import { beforeEach, expect, test, vi } from 'vitest';
import { attachTopic, createTopic, detachTopic } from './topics';
import { UNEXPECTED } from '$lib/shared/lib/api-failure';

/**
 * The editor's topic chips.
 *
 * §03: "topics · combobox · own topics only · **Each chip is its own request;
 * one failure doesn't roll back the others.**" So each of these is one call
 * about one chip, and none of them knows about the others.
 */

const fetchFn = vi.fn<typeof fetch>();

function answers(status: number, body: unknown = null) {
	fetchFn.mockResolvedValue(
		new Response(body === null ? null : JSON.stringify(body), {
			status,
			headers: { 'content-type': 'application/json' }
		})
	);
}

beforeEach(() => fetchFn.mockReset());

test('attaching names the post in the path and the topic in the body', async () => {
	answers(204);

	expect(await attachTopic('post-1', 't-rust', fetchFn)).toEqual({ ok: true });
	expect(fetchFn.mock.calls[0][0]).toBe('/api/blog/post-1/topics');
	expect(fetchFn.mock.calls[0][1]?.method).toBe('POST');
	expect(JSON.parse(String(fetchFn.mock.calls[0][1]?.body))).toEqual({ topic_id: 't-rust' });
});

test('detaching is the same address with the other verb', async () => {
	answers(204);

	expect(await detachTopic('post-1', 't-rust', fetchFn)).toEqual({ ok: true });
	expect(fetchFn.mock.calls[0][1]?.method).toBe('DELETE');
	expect(JSON.parse(String(fetchFn.mock.calls[0][1]?.body))).toEqual({ topic_id: 't-rust' });
});

test('escapes an id rather than pasting it into the path', async () => {
	answers(204);

	await attachTopic('a/b', 't-1', fetchFn);

	expect(fetchFn.mock.calls[0][0]).toBe('/api/blog/a%2Fb/topics');
});

test('creating a topic hands back the one it made, ready to attach', async () => {
	// §03: "Create topic (inline) · title, description · Create & attach".
	// Creating and attaching are two requests, and this is the first.
	answers(201, { data: { id: 't-new', title: 'Rust' } });

	expect(await createTopic('Rust', fetchFn)).toEqual({
		ok: true,
		topic: { id: 't-new', title: 'Rust' }
	});
	expect(fetchFn.mock.calls[0][0]).toBe('/api/topics');
	expect(JSON.parse(String(fetchFn.mock.calls[0][1]?.body))).toEqual({ title: 'Rust' });
});

test('a refused attach says so, and says nothing about the other chips', async () => {
	answers(409, { error: { code: 'TOPIC_ALREADY_ATTACHED' } });

	const result = await attachTopic('post-1', 't-rust', fetchFn);

	expect(result.ok).toBe(false);
});

test('an unreachable backend is a failure, not a silent success', async () => {
	// Built here rather than by resetting the shared mock: Vitest tracks a
	// `vi.fn`'s settled result too, and a rejection stored on a reused mock is
	// reported as unhandled even once the call has caught it.
	const unreachable = vi.fn<typeof fetch>(async () => {
		throw new TypeError('Failed to fetch');
	});

	expect(await attachTopic('post-1', 't-rust', unreachable)).toEqual({
		ok: false,
		message: UNEXPECTED,
		kind: 'notOurs'
	});
});

test('a created topic that comes back without an id is not a topic', async () => {
	answers(201, { data: {} });

	expect((await createTopic('Rust', fetchFn)).ok).toBe(false);
});
