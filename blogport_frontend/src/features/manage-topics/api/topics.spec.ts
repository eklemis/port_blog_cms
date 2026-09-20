import { expect, test, vi } from 'vitest';
import { renameTopic, retireTopic, topicUsage } from './topics';

/**
 * Managing the shared vocabulary.
 *
 * §02's journey says "Retire, don't rename — only soft delete exists... No
 * update endpoint. A typo is unfixable." That is no longer true:
 * `PATCH /api/topics/{id}` exists, and its own description says why — "a typo
 * in a title was permanent and visible on every tagged post and project. The
 * workaround was create-retag-retire, by hand." The topic keeps its id, so
 * nothing needs retagging. Raised with the designer.
 */

const ok = (body: unknown, status = 200) =>
	new Response(JSON.stringify(body), { status, headers: { 'content-type': 'application/json' } });

const sent = (mock: { mock: { calls: unknown[] } }) =>
	mock.mock.calls as unknown as [string, RequestInit][];

test('a rename sends only the title, and keeps the id', async () => {
	// The id is the whole point: everything tagged follows the new name rather
	// than needing to be retagged.
	const fetchFn = vi.fn(async () => ok({ data: { id: 't-1', title: 'Rust' } }));

	const result = await renameTopic('t-1', 'Rust', fetchFn as unknown as typeof fetch);

	expect(result).toMatchObject({ ok: true });

	const [url, init] = sent(fetchFn)[0];
	expect(url).toBe('/api/topics/t-1');
	expect(init.method).toBe('PATCH');
	expect(JSON.parse(String(init.body))).toEqual({ title: 'Rust' });
});

test('renaming to a name you already have is a near-miss, not a failure', async () => {
	// §02: "TOPIC_ALREADY_EXISTS · 409. In the inline creator this is a
	// near-miss, not a failure." The same is true here.
	const fetchFn = vi.fn(async () => ok({ error: { code: 'TOPIC_ALREADY_EXISTS' } }, 409));

	const result = await renameTopic('t-1', 'Rust', fetchFn as unknown as typeof fetch);

	expect(result).toMatchObject({
		ok: false,
		kind: 'collision',
		message: 'You already have a topic called Rust.'
	});
});

test('retiring soft-deletes, and says so by the verb it uses', async () => {
	const fetchFn = vi.fn(async () => ok({ data: null }));

	await expect(retireTopic('t-1', fetchFn as unknown as typeof fetch)).resolves.toMatchObject({
		ok: true
	});

	const [url, init] = sent(fetchFn)[0];
	expect(url).toBe('/api/topics/t-1');
	expect(init.method).toBe('DELETE');
});

test('a topic that is not yours is refused, not hidden', async () => {
	// The endpoint rejects another user's topic as forbidden rather than
	// not-found, so the console should not report it as missing either.
	const fetchFn = vi.fn(async () => ok({ error: { code: 'FORBIDDEN' } }, 403));

	await expect(retireTopic('t-1', fetchFn as unknown as typeof fetch)).resolves.toMatchObject({
		ok: false,
		kind: 'gate'
	});
});

test('usage comes back as the two real counts', async () => {
	const fetchFn = vi.fn(async () => ok({ data: { posts: 6, projects: 2 } }));

	await expect(topicUsage('t-1', fetchFn as unknown as typeof fetch)).resolves.toEqual({
		posts: 6,
		projects: 2
	});
});

test('usage that cannot be counted is null, never zero', async () => {
	// Zero would read as "nothing is using it" and invite a confident retire.
	const fetchFn = vi.fn(async () => {
		throw new Error('offline');
	});

	await expect(topicUsage('t-1', fetchFn as unknown as typeof fetch)).resolves.toBe(null);
});

test('a usage response missing a count is not half-trusted', async () => {
	const fetchFn = vi.fn(async () => ok({ data: { posts: 6 } }));

	await expect(topicUsage('t-1', fetchFn as unknown as typeof fetch)).resolves.toBe(null);
});
