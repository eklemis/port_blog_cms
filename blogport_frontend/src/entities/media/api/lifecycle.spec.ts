import { expect, test, vi } from 'vitest';
import { archiveMedia, purgeMedia, restoreMedia } from './lifecycle';

/**
 * The archive ladder for one image.
 *
 * §02's state table gives media the same three rungs as everything else:
 * `DELETE` soft, `POST …/restore`, `DELETE …/hard`. The console calls the
 * first "archive" for the same reason it calls a topic's "retire" — the word
 * has to match how reversible the act is.
 */

const ok = (body: unknown, status = 200) =>
	new Response(JSON.stringify(body), { status, headers: { 'content-type': 'application/json' } });

const sent = (mock: { mock: { calls: unknown[] } }) =>
	mock.mock.calls as unknown as [string, RequestInit | undefined][];

test('archiving is a soft delete', async () => {
	const fetchFn = vi.fn(async () => ok({ data: null }));

	await expect(archiveMedia('m-1', fetchFn as unknown as typeof fetch)).resolves.toMatchObject({
		ok: true
	});

	const [url, init] = sent(fetchFn)[0];
	expect(url).toBe('/api/media/m-1');
	expect(init?.method).toBe('DELETE');
});

test('restoring puts it back', async () => {
	const fetchFn = vi.fn(async () => ok({ data: null }));

	await restoreMedia('m-1', fetchFn as unknown as typeof fetch);

	const [url, init] = sent(fetchFn)[0];
	expect(url).toBe('/api/media/m-1/restore');
	expect(init?.method).toBe('POST');
});

test('purging is the one that does not come back', async () => {
	const fetchFn = vi.fn(async () => ok({ data: null }));

	await purgeMedia('m-1', fetchFn as unknown as typeof fetch);

	const [url, init] = sent(fetchFn)[0];
	expect(url).toBe('/api/media/m-1/hard');
	expect(init?.method).toBe('DELETE');
});

test('an id is escaped rather than pasted into a path', async () => {
	const fetchFn = vi.fn(async () => ok({ data: null }));

	await archiveMedia('a/b', fetchFn as unknown as typeof fetch);

	expect(sent(fetchFn)[0][0]).toBe('/api/media/a%2Fb');
});

test('a refusal is reported rather than looking done', async () => {
	const fetchFn = vi.fn(async () => ok({ error: { code: 'MEDIA_NOT_FOUND' } }, 404));

	await expect(archiveMedia('m-1', fetchFn as unknown as typeof fetch)).resolves.toMatchObject({
		ok: false
	});
});
