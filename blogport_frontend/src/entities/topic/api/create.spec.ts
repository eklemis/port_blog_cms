import { expect, test, vi } from 'vitest';
import { createTopic } from './create';

/**
 * `POST /api/topics` — the inline creator, and the Topics screen's own.
 *
 * §02 asks for both halves together: "title and description in the same
 * popover — a taxonomy of bare words stops being useful at about fifteen
 * entries."
 */

const ok = (body: unknown, status = 200) =>
	new Response(JSON.stringify(body), { status, headers: { 'content-type': 'application/json' } });

const sent = (mock: { mock: { calls: unknown[] } }) =>
	mock.mock.calls as unknown as [string, RequestInit][];

test('sends the title, and the description when there is one', () => {
	const fetchFn = vi.fn(async () => ok({ data: { id: 't-1', title: 'Rust' } }, 201));

	return createTopic('Rust', fetchFn as unknown as typeof fetch, 'Systems work.').then(() => {
		expect(JSON.parse(String(sent(fetchFn)[0][1].body))).toEqual({
			title: 'Rust',
			description: 'Systems work.'
		});
	});
});

test('a topic with no description sends none rather than an empty one', () => {
	// The editor rail's inline creator has nowhere to type one.
	const fetchFn = vi.fn(async () => ok({ data: { id: 't-1', title: 'Rust' } }, 201));

	return createTopic('Rust', fetchFn as unknown as typeof fetch).then(() => {
		expect(JSON.parse(String(sent(fetchFn)[0][1].body))).toEqual({ title: 'Rust' });
	});
});

test('a name you already have is a near-miss, and says which name', () => {
	// §02: "TOPIC_ALREADY_EXISTS · 409. In the inline creator this is a
	// near-miss, not a failure: 'You already have a topic called Rust'."
	const fetchFn = vi.fn(async () => ok({ error: { code: 'TOPIC_ALREADY_EXISTS' } }, 409));

	return createTopic('Rust', fetchFn as unknown as typeof fetch).then((result) => {
		expect(result).toMatchObject({
			ok: false,
			kind: 'collision',
			message: 'You already have a topic called Rust.'
		});
	});
});
