import { expect, test, vi } from 'vitest';
import { UNEXPECTED } from '$lib/shared/lib/api-failure';
import { SLUG_TAKEN, checkSlug, createPost } from './create-post';

/**
 * Creating the draft — J4 step one.
 *
 * The post has to exist before it can have a cover image: media attaches to a
 * `target_id`. So this call is small on purpose and everything else is the
 * editor's.
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

const DRAFT = { title: 'Building a CMS', slug: 'building-a-cms', content: 'The first line.' };

test('sends the three fields the API requires, and no published_at', async () => {
	// Omitting `published_at` is what makes it a draft. Sending null would be a
	// different thing to say and the API reads it as clearing a date.
	const fetchFn = respond(201, { id: 'post-1', slug: 'building-a-cms' });

	await createPost(DRAFT, fetchFn);

	const body = JSON.parse(fetchFn.mock.calls[0][1]?.body as string);
	expect(body).toEqual(DRAFT);
	expect('published_at' in body).toBe(false);
});

test('the new post’s id comes back, because the editor is the next screen', async () => {
	const fetchFn = respond(201, { id: 'post-1', slug: 'building-a-cms' });

	expect(await createPost(DRAFT, fetchFn)).toEqual({ ok: true, id: 'post-1' });
});

test('a taken slug is a field failure with the address kept', async () => {
	// J4: mark the field, and never lose the draft body to a slug collision.
	const fetchFn = respond(409, {
		error: { code: 'SLUG_ALREADY_EXISTS', message: 'Slug already exists' }
	});

	expect(await createPost(DRAFT, fetchFn)).toEqual({
		ok: false,
		field: 'slug',
		message: SLUG_TAKEN,
		retryAfterSeconds: null,
		kind: 'collision'
	});
});

test('a refused title lands under the title, in the server’s words', async () => {
	const fetchFn = respond(400, {
		error: { code: 'INVALID_TITLE', message: 'Title must not exceed 200 characters' }
	});

	expect(await createPost(DRAFT, fetchFn)).toMatchObject({
		field: 'title',
		message: 'Title must not exceed 200 characters',
		kind: 'field'
	});
});

test('a refused body lands under the body', async () => {
	const fetchFn = respond(400, {
		error: { code: 'INVALID_CONTENT', message: 'Content cannot be empty' }
	});

	expect(await createPost(DRAFT, fetchFn)).toMatchObject({ field: 'content', kind: 'field' });
});

test('a rate limit counts down against the real Retry-After', async () => {
	const fetchFn = respond(429, { error: { code: 'RATE_LIMITED' } }, { 'retry-after': '120' });

	expect(await createPost(DRAFT, fetchFn)).toMatchObject({
		field: null,
		retryAfterSeconds: 120,
		kind: 'wait'
	});
});

test('anything else is ours to own rather than theirs to decipher', async () => {
	const fetchFn = respond(500, { error: { code: 'INTERNAL_ERROR' } });

	expect(await createPost(DRAFT, fetchFn)).toMatchObject({
		field: null,
		message: UNEXPECTED,
		kind: 'notOurs'
	});
});

test('a dead network is not a raw exception in the user’s face', async () => {
	const fetchFn = vi.fn<typeof fetch>(async () => {
		throw new TypeError('Failed to fetch');
	});

	expect(await createPost(DRAFT, fetchFn)).toMatchObject({ message: UNEXPECTED });
});

// ── checking ahead of time ─────────────────────────────────────────────────

test('a free address reports free', async () => {
	const fetchFn = respond(200, { available: true, slug: 'a-post', suggestion: null });

	expect(await checkSlug('a-post', fetchFn)).toEqual({ available: true, suggestion: null });
});

test('a taken address carries the free variant the backend found', async () => {
	// Not a "-2" invented here: an address that is actually free.
	const fetchFn = respond(200, { available: false, slug: 'a-post', suggestion: 'a-post-2' });

	expect(await checkSlug('a-post', fetchFn)).toEqual({
		available: false,
		suggestion: 'a-post-2'
	});
});

test('a check that cannot be made never blocks the form', async () => {
	const fetchFn = vi.fn<typeof fetch>(async () => {
		throw new TypeError('Failed to fetch');
	});

	expect(await checkSlug('a-post', fetchFn)).toEqual({ available: true, suggestion: null });
});
