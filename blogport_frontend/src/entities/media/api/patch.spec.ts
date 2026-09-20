import { expect, test, vi } from 'vitest';
import { patchMedia } from './patch';

/**
 * `PATCH /api/media/{id}` — the one call two features need.
 *
 * The post editor corrects a cover's alt text; the project editor moves a
 * screenshot. Slices in the same layer may not import each other, so the shared
 * behaviour lives below both rather than being written twice.
 */

const ok = (body: unknown, status = 200) =>
	new Response(JSON.stringify(body), { status, headers: { 'content-type': 'application/json' } });

const sent = (mock: { mock: { calls: unknown[] } }) =>
	mock.mock.calls as unknown as [string, RequestInit][];

test('sends only the fields it was given', async () => {
	// PATCH changes what is present. A key sent as null clears the value, so an
	// unasked-for key is a silent deletion.
	const fetchFn = vi.fn(async () => ok({ data: null }));

	await patchMedia('m-1', { position: 2 }, fetchFn as unknown as typeof fetch);

	const [url, init] = sent(fetchFn)[0];
	expect(url).toBe('/api/media/m-1');
	expect(init.method).toBe('PATCH');
	expect(JSON.parse(String(init.body))).toEqual({ position: 2 });
});

test('escapes the id rather than pasting it into a path', async () => {
	const fetchFn = vi.fn(async () => ok({ data: null }));

	await patchMedia('a/b', { position: 0 }, fetchFn as unknown as typeof fetch);

	expect(sent(fetchFn)[0][0]).toBe('/api/media/a%2Fb');
});

test('a refusal is reported rather than looking saved', async () => {
	const fetchFn = vi.fn(async () => ok({ error: { code: 'MEDIA_NOT_FOUND' } }, 404));

	await expect(
		patchMedia('m-1', { position: 1 }, fetchFn as unknown as typeof fetch)
	).resolves.toMatchObject({ ok: false });
});

test('a network that never answered is a failure, not a hang', async () => {
	const fetchFn = vi.fn(async () => {
		throw new Error('offline');
	});

	await expect(
		patchMedia('m-1', { position: 1 }, fetchFn as unknown as typeof fetch)
	).resolves.toMatchObject({ ok: false, kind: 'notOurs' });
});
