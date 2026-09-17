import { expect, test, vi } from 'vitest';
import { UNEXPECTED } from '$lib/shared/lib/api-failure';
import { GONE } from './archive';
import { archivePost, bulkPosts, purgePost, restorePost } from './archive';

/**
 * The two ways out of the archive — §06's second and third rungs.
 *
 * Neither is optimistic. Restore is reversible but the row leaving the list is
 * the whole of its feedback; purge is irreversible, and optimism about that is
 * just an inaccurate screen.
 */

const respond = (status: number, body: unknown = null) =>
	vi.fn<typeof fetch>(async () =>
		body === null
			? new Response(null, { status })
			: new Response(JSON.stringify(body), {
					status,
					headers: { 'content-type': 'application/json' }
				})
	);

test('restoring posts to the restore route for that post', async () => {
	const fetchFn = respond(204);

	expect(await restorePost('post-1', fetchFn)).toEqual({ ok: true });
	expect(fetchFn.mock.calls[0][0]).toBe('/api/blog/post-1/restore');
	expect(fetchFn.mock.calls[0][1]?.method).toBe('POST');
});

test('purging deletes at the hard route, escaped', async () => {
	const fetchFn = respond(204);

	expect(await purgePost('a/b', fetchFn)).toEqual({ ok: true });
	expect(fetchFn.mock.calls[0][0]).toBe('/api/blog/a%2Fb/hard');
	expect(fetchFn.mock.calls[0][1]?.method).toBe('DELETE');
});

test('archiving deletes the post, which is what archiving is on the wire', async () => {
	const fetchFn = respond(204);

	expect(await archivePost('post-1', fetchFn)).toEqual({ ok: true });
	expect(fetchFn.mock.calls[0][0]).toBe('/api/blog/post-1');
	expect(fetchFn.mock.calls[0][1]?.method).toBe('DELETE');
});

test('a post that is no longer there is not found, not our failure', async () => {
	// Restored or purged in another tab. The list is stale, not broken.
	const fetchFn = respond(404, { error: { code: 'POST_NOT_FOUND' } });

	expect(await restorePost('post-1', fetchFn)).toEqual({
		ok: false,
		message: 'That post is no longer in the archive.',
		kind: 'notFound'
	});
});

test('anything else is ours to own rather than theirs to decipher', async () => {
	const fetchFn = respond(500, { error: { code: 'INTERNAL_ERROR' } });

	expect(await purgePost('post-1', fetchFn)).toEqual({
		ok: false,
		message: UNEXPECTED,
		kind: 'notOurs'
	});
});

test('a dead network is a failure to report, not an exception', async () => {
	const fetchFn = vi.fn<typeof fetch>(async () => {
		throw new TypeError('Failed to fetch');
	});

	expect(await restorePost('post-1', fetchFn)).toMatchObject({ ok: false, kind: 'notOurs' });
});

// ── several at once ────────────────────────────────────────────────────────

test('a batch names its operation and its ids', async () => {
	const fetchFn = vi.fn<typeof fetch>(
		async () =>
			new Response(JSON.stringify({ succeeded: ['a', 'b'], failed: [] }), {
				status: 200,
				headers: { 'content-type': 'application/json' }
			})
	);

	await bulkPosts('restore', ['a', 'b'], fetchFn);

	expect(fetchFn.mock.calls[0][0]).toBe('/api/blog/bulk');
	expect(JSON.parse(fetchFn.mock.calls[0][1]?.body as string)).toEqual({
		op: 'restore',
		ids: ['a', 'b']
	});
});

test('a partial batch reports each failure with its own reason', async () => {
	// "success: true means the batch ran, not that every item did." Each failed
	// row keeps a sentence of its own, classified by its own code.
	const fetchFn = vi.fn<typeof fetch>(
		async () =>
			new Response(
				JSON.stringify({
					succeeded: ['a'],
					failed: [
						{ id: 'b', code: 'POST_NOT_FOUND', message: 'x' },
						{ id: 'c', code: 'INTERNAL_ERROR', message: 'x' }
					]
				}),
				{ status: 200, headers: { 'content-type': 'application/json' } }
			)
	);

	expect(await bulkPosts('hard_delete', ['a', 'b', 'c'], fetchFn)).toEqual({
		ok: true,
		succeeded: ['a'],
		failed: { b: GONE, c: UNEXPECTED }
	});
});

test('a batch that could not run at all is one failure for all of it', async () => {
	const fetchFn = vi.fn<typeof fetch>(async () => {
		throw new TypeError('Failed to fetch');
	});

	expect(await bulkPosts('restore', ['a'], fetchFn)).toEqual({
		ok: false,
		message: UNEXPECTED,
		kind: 'notOurs'
	});
});

test('names every operation the server publishes, not the ones one screen uses', async () => {
	// The backend's enum carried six ops; the prose this union was typed from
	// listed five, so `unpublish` was missing from the type rather than merely
	// unused. The union comes from the generated schema now, and this is the op
	// that proves it — it would not have compiled before.
	const fetchFn = vi.fn<typeof fetch>(
		async () =>
			new Response(JSON.stringify({ succeeded: ['a'], failed: [] }), {
				status: 200,
				headers: { 'content-type': 'application/json' }
			})
	);

	await bulkPosts('unpublish', ['a'], fetchFn);

	expect(JSON.parse(String(fetchFn.mock.calls[0][1]?.body))).toEqual({
		op: 'unpublish',
		ids: ['a']
	});
});
