import { expect, test, vi } from 'vitest';
import { UNEXPECTED } from '$lib/shared/lib/api-failure';
import { purgePost, restorePost } from './archive';

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
