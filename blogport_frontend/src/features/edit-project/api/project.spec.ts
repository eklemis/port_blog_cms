import { expect, test, vi } from 'vitest';
import { attachTopic, detachTopic, patchProject } from './project';

/**
 * Saving a project, and putting topics on it.
 *
 * `PATCH /api/projects/{id}` is field-level — §02: "there is no full-replace
 * PUT, so the editor is field-level throughout and never sends an object it did
 * not load first."
 */

const ok = (body: unknown, status = 200) =>
	new Response(JSON.stringify(body), { status, headers: { 'content-type': 'application/json' } });

const sent = (mock: { mock: { calls: unknown[] } }) =>
	mock.mock.calls as unknown as [string, RequestInit][];

test('sends only the fields that changed', async () => {
	const fetchFn = vi.fn(async () => ok({ data: null }));

	await patchProject('p-1', { title: 'Blogport CMS' }, fetchFn as unknown as typeof fetch);

	const [url, init] = sent(fetchFn)[0];
	expect(url).toBe('/api/projects/p-1');
	expect(init.method).toBe('PATCH');
	expect(JSON.parse(String(init.body))).toEqual({ title: 'Blogport CMS' });
});

test('an emptied URL is cleared rather than sent as an empty string', async () => {
	// `null` clears it, per the endpoint. An empty string would be stored and
	// then rendered as a link to nowhere.
	const fetchFn = vi.fn(async () => ok({ data: null }));

	await patchProject('p-1', { repo_url: null }, fetchFn as unknown as typeof fetch);

	expect(JSON.parse(String(sent(fetchFn)[0][1].body))).toEqual({ repo_url: null });
});

test('a slug collision is reported as the field problem it is', async () => {
	const fetchFn = vi.fn(async () => ok({ error: { code: 'SLUG_ALREADY_EXISTS' } }, 409));

	const result = await patchProject('p-1', { title: 'x' }, fetchFn as unknown as typeof fetch);

	expect(result).toMatchObject({ ok: false, kind: 'collision' });
});

test('each topic is its own request, so one failure does not undo the others', async () => {
	// §03's rule for the chips, and the same here.
	const fetchFn = vi.fn(async () => ok({ data: null }));

	await attachTopic('p-1', 't-1', fetchFn as unknown as typeof fetch);
	await detachTopic('p-1', 't-2', fetchFn as unknown as typeof fetch);

	const calls = sent(fetchFn);
	expect(calls[0][0]).toBe('/api/projects/p-1/topics');
	expect(calls[0][1].method).toBe('POST');
	expect(calls[1][1].method).toBe('DELETE');
	expect(JSON.parse(String(calls[1][1].body))).toEqual({ topic_id: 't-2' });
});

test('an unreachable server is a failure the form can show', async () => {
	const fetchFn = vi.fn(async () => {
		throw new Error('offline');
	});

	await expect(
		patchProject('p-1', { title: 'x' }, fetchFn as unknown as typeof fetch)
	).resolves.toMatchObject({ ok: false, kind: 'notOurs' });
});
