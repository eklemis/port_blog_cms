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

/**
 * The blog list's answer. Topics answer separately and always succeed unless a
 * test says otherwise — they are the filter's options, not the list itself.
 */
function page(body: unknown, ok = true, topics: unknown = { data: [] }) {
	fetchImpl.mockImplementation(async (_event: unknown, path: string) =>
		String(path).startsWith('/api/topics')
			? { ok: true, json: async () => topics }
			: { ok, json: async () => body }
	);
}

const rows = { data: { items: [{ id: '1', title: 'A post' }], page: 1, per_page: 10, total: 24 } };

function event(query = '') {
	return { url: new URL(`http://localhost/studio/posts${query}`) };
}

/** The path the loader asked for the rows with. */
function askedFor() {
	const call = fetchImpl.mock.calls.find((entry) => String(entry[1]).startsWith('/api/blog'));
	return String(call?.[1]);
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

// ── topics ─────────────────────────────────────────────────────────────────

test('carries the topic to the API, and fetches the options to choose from', async () => {
	page(rows, true, { data: [{ id: 'topic-1', title: 'Rust', description: '' }] });

	const data = await load(event('?topic_id=topic-1') as never);

	expect(askedFor()).toContain('topic_id=topic-1');
	expect(data).toMatchObject({ topics: [{ id: 'topic-1', title: 'Rust' }], topic: 'topic-1' });
});

test('a topic is a filter, so filtered-empty stays distinguishable', async () => {
	page(rows);

	expect(await load(event('?topic_id=topic-1') as never)).toMatchObject({ filtered: true });
});

test('losing the topics loses the control, not the list', async () => {
	// A select with no options is a dead control; the rows are what matter.
	fetchImpl.mockImplementation(async (_event: unknown, path: string) =>
		String(path).startsWith('/api/topics')
			? { ok: false, json: async () => ({}) }
			: { ok: true, json: async () => rows }
	);

	const data = await load(event() as never);

	expect(data).toMatchObject({ topics: [], failed: false });
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
