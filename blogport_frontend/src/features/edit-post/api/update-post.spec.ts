import { expect, test, vi } from 'vitest';
import { UNEXPECTED } from '$lib/shared/lib/api-failure';
import { SLUG_TAKEN, patchPost } from './update-post';

/**
 * Saving an edit — the call the autosave loop makes.
 *
 * PATCH only, and only the keys that moved: the API changes what is present and
 * leaves out what is not, so the editor never sends an object it did not load.
 */

function respond(status: number, body: unknown, headers: Record<string, string> = {}) {
	return vi.fn<typeof fetch>(
		async () =>
			new Response(JSON.stringify(body), {
				status,
				headers: { 'content-type': 'application/json', ...headers }
			})
	);
}

test('sends the changes to the post it names', async () => {
	const fetchFn = respond(200, { id: 'post-1' });

	await patchPost('post-1', { title: 'New title' }, fetchFn);

	expect(fetchFn.mock.calls[0][0]).toBe('/api/blog/post-1');
	expect(fetchFn.mock.calls[0][1]?.method).toBe('PATCH');
	expect(JSON.parse(fetchFn.mock.calls[0][1]?.body as string)).toEqual({ title: 'New title' });
});

test('the id is escaped rather than pasted into a path', async () => {
	const fetchFn = respond(200, {});

	await patchPost('a/b', { title: 'x' }, fetchFn);

	expect(fetchFn.mock.calls[0][0]).toBe('/api/blog/a%2Fb');
});

test('a save that lands says so, with nothing else to report', async () => {
	const fetchFn = respond(200, { id: 'post-1' });

	expect(await patchPost('post-1', { title: 'x' }, fetchFn)).toEqual({ ok: true });
});

test('a taken address is a field failure, under the field', async () => {
	const fetchFn = respond(409, { error: { code: 'SLUG_ALREADY_EXISTS' } });

	expect(await patchPost('post-1', { slug: 'taken' }, fetchFn)).toEqual({
		ok: false,
		field: 'slug',
		message: SLUG_TAKEN,
		kind: 'collision'
	});
});

test('a refused title lands under the title, in the server’s words', async () => {
	const fetchFn = respond(400, {
		error: { code: 'INVALID_TITLE', message: 'Title must not exceed 200 characters' }
	});

	expect(await patchPost('post-1', { title: 'x' }, fetchFn)).toMatchObject({
		field: 'title',
		message: 'Title must not exceed 200 characters'
	});
});

test('someone else’s post is the gate, not a field', async () => {
	// J4 calls this POST_UNAUTHORIZED; the API says a post belonging to another
	// author is reported as not found. Both land here — see the PR.
	const fetchFn = respond(404, { error: { code: 'POST_NOT_FOUND' } });

	expect(await patchPost('post-1', { title: 'x' }, fetchFn)).toMatchObject({
		field: null,
		kind: 'gate'
	});
});

test('anything else is ours to own rather than theirs to decipher', async () => {
	const fetchFn = respond(500, { error: { code: 'INTERNAL_ERROR' } });

	expect(await patchPost('post-1', { title: 'x' }, fetchFn)).toMatchObject({
		message: UNEXPECTED,
		kind: 'notOurs'
	});
});

test('a dead network is a failure to report, not an exception to throw', async () => {
	// The autosave loop reads this and retries; it must never see a rejection.
	const fetchFn = vi.fn<typeof fetch>(async () => {
		throw new TypeError('Failed to fetch');
	});

	expect(await patchPost('post-1', { title: 'x' }, fetchFn)).toMatchObject({
		ok: false,
		message: UNEXPECTED
	});
});
