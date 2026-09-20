import { expect, test, vi } from 'vitest';
import { SLUG_TAKEN, checkSlug, createProject } from './create-project';

/**
 * Creating a project — and unlike a post, creating one publishes it.
 *
 * §02: "Projects have no `published_at`. Creating one publishes it." There is
 * no draft to come back to, which is why the form says so before the button is
 * pressed rather than after.
 */

const ok = (body: unknown, status = 200) =>
	new Response(JSON.stringify(body), { status, headers: { 'content-type': 'application/json' } });

const sent = (mock: { mock: { calls: unknown[] } }) =>
	mock.mock.calls as unknown as [string, RequestInit][];

test('sends what a project needs to exist', async () => {
	const fetchFn = vi.fn(async () => ok({ data: { id: 'p-1' } }, 201));

	const result = await createProject(
		{ title: 'Blogport CMS', slug: 'blogport-cms', description: 'A portfolio CMS.' },
		fetchFn as unknown as typeof fetch
	);

	expect(result).toEqual({ ok: true, id: 'p-1' });

	const [url, init] = sent(fetchFn)[0];
	expect(url).toBe('/api/projects');
	expect(JSON.parse(String(init.body))).toEqual({
		title: 'Blogport CMS',
		slug: 'blogport-cms',
		description: 'A portfolio CMS.'
	});
});

test('a created project with no id is a failure, not a project', async () => {
	// There would be nowhere to send the person next.
	const fetchFn = vi.fn(async () => ok({ data: {} }, 201));

	const result = await createProject(
		{ title: 'x', slug: 'x', description: 'y' },
		fetchFn as unknown as typeof fetch
	);

	expect(result.ok).toBe(false);
});

test('a taken address is reported under the address, in our words', async () => {
	// The backend's sentence is written for a log. This one is read by someone
	// who has just typed an address.
	const fetchFn = vi.fn(async () =>
		ok({ error: { code: 'SLUG_ALREADY_EXISTS', message: 'Slug already exists' } }, 409)
	);

	const result = await createProject(
		{ title: 'x', slug: 'taken', description: 'y' },
		fetchFn as unknown as typeof fetch
	);

	expect(result).toMatchObject({
		ok: false,
		field: 'slug',
		message: SLUG_TAKEN,
		kind: 'collision'
	});
});

test('a refused field is named, so the error lands under it', async () => {
	const fetchFn = vi.fn(async () =>
		ok({ error: { code: 'INVALID_TITLE', message: 'Title must not be empty' } }, 400)
	);

	const result = await createProject(
		{ title: '', slug: 'x', description: 'y' },
		fetchFn as unknown as typeof fetch
	);

	expect(result).toMatchObject({ ok: false, field: 'title', kind: 'field' });
});

test('an unreachable server is not blamed on the person', async () => {
	const fetchFn = vi.fn(async () => {
		throw new Error('offline');
	});

	const result = await createProject(
		{ title: 'x', slug: 'x', description: 'y' },
		fetchFn as unknown as typeof fetch
	);

	expect(result).toMatchObject({ ok: false, field: null, kind: 'notOurs' });
});

test('an address check that cannot be made reports the address free', async () => {
	// A courtesy ahead of the real answer, which is SLUG_ALREADY_EXISTS on
	// create. Reporting "taken" would stop someone submitting a free address.
	const fetchFn = vi.fn(async () => {
		throw new Error('offline');
	});

	await expect(checkSlug('blogport-cms', fetchFn as unknown as typeof fetch)).resolves.toEqual({
		available: true,
		suggestion: null
	});
});

test('a taken address comes back with the free one the server suggests', async () => {
	// The suggestion is an address that is actually free, which a guess is not.
	const fetchFn = vi.fn(async () => ok({ available: false, suggestion: 'blogport-cms-2' }));

	await expect(checkSlug('blogport-cms', fetchFn as unknown as typeof fetch)).resolves.toEqual({
		available: false,
		suggestion: 'blogport-cms-2'
	});
});
