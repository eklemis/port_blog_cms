import { beforeEach, expect, test, vi } from 'vitest';

const fetchImpl = vi.fn();

/**
 * Set to make the call throw. A plain variable rather than a rejecting mock:
 * Vitest tracks a `vi.fn`'s settled result too, and that second promise is
 * reported as unhandled even when the loader catches the first.
 */
let unreachable: Error | null = null;

vi.mock('$lib/shared/api/backend.server', () => ({
	authenticatedFetch: async (...args: unknown[]) => {
		if (unreachable) throw unreachable;
		return fetchImpl(...args);
	}
}));

const { load } = await import('./+page.server');

/**
 * The posts list.
 *
 * Defaults are page=1, per_page=10 and the resource's own sort, and every
 * filter lives in the URL — so a filtered list is a shareable, reloadable,
 * back-button-safe address.
 */

function page(body: unknown, ok = true) {
	fetchImpl.mockResolvedValue({ ok, json: async () => body });
}

const rows = { data: { items: [{ id: '1', title: 'A post' }], page: 1, per_page: 10, total: 24 } };

function event(query = '') {
	return { url: new URL(`http://localhost/studio/posts${query}`) };
}

/** The path the loader asked the backend for. */
function askedFor() {
	return String(fetchImpl.mock.calls[0][1]);
}

beforeEach(() => {
	fetchImpl.mockReset();
	unreachable = null;
});

test('asks for the first page with the blueprint’s defaults', async () => {
	page(rows);

	await load(event() as never);

	expect(askedFor()).toContain('per_page=10');
	expect(askedFor()).toContain('page=1');
});

test('hands the rows and the totals through', async () => {
	page(rows);

	const data = await load(event() as never);

	expect(data).toMatchObject({ posts: [{ title: 'A post' }], total: 24, page: 1, perPage: 10 });
});

test('carries the search term to the API', async () => {
	page(rows);

	await load(event('?search=rust') as never);

	expect(askedFor()).toContain('search=rust');
});

test('a blank search is not a filter', async () => {
	// Otherwise `?search=` narrows nothing and still calls the list filtered.
	page(rows);

	const data = await load(event('?search=%20%20') as never);

	expect(askedFor()).not.toContain('search=');
	expect(data).toMatchObject({ filtered: false });
});

test('knows when the list is narrowed, so empty can be told from filtered-empty', async () => {
	// Conflating them tells someone with 24 posts that they have none.
	page(rows);

	expect(await load(event('?search=rust') as never)).toMatchObject({ filtered: true });
	fetchImpl.mockReset();
	page(rows);
	expect(await load(event('?published=true') as never)).toMatchObject({ filtered: true });
	fetchImpl.mockReset();
	page(rows);
	expect(await load(event() as never)).toMatchObject({ filtered: false });
});

test('sort travels too, and an unknown one is ignored rather than sent', async () => {
	page(rows);
	await load(event('?sort=updated_newest') as never);
	expect(askedFor()).toContain('sort=updated_newest');

	fetchImpl.mockReset();
	page(rows);
	await load(event('?sort=whatever-they-typed') as never);
	expect(askedFor()).not.toContain('sort=');
});

test('a page number that is not one is honoured', async () => {
	page(rows);

	await load(event('?page=3') as never);

	expect(askedFor()).toContain('page=3');
});

test('nonsense paging falls back rather than asking for page NaN', async () => {
	page(rows);

	await load(event('?page=-2') as never);

	expect(askedFor()).toContain('page=1');
});

test('a failed fetch is a state, not an exception', async () => {
	// Four states, always: this is the fourth, and it says the data is safe.
	page({}, false);

	expect(await load(event() as never)).toMatchObject({ failed: true, posts: [] });
});

test('an unreachable backend is the same state', async () => {
	unreachable = new TypeError('fetch failed');

	expect(await load(event() as never)).toMatchObject({ failed: true, posts: [] });
});
