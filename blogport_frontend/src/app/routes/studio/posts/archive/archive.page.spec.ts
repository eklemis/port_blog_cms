import { beforeEach, expect, test, vi } from 'vitest';

const fetchImpl = vi.fn();
let unreachable: Error | null = null;

vi.mock('$lib/shared/api/backend.server', () => ({
	authenticatedFetch: async (...args: unknown[]) => {
		if (unreachable) throw unreachable;
		return fetchImpl(...args);
	}
}));

const { load } = await import('./+page.server');

/**
 * `/studio/posts/archive` — archived posts, restore or purge.
 *
 * `?deleted=true` on the same listing, which the backend added for this screen:
 * until then the owner query hard-coded live posts and the screen had two
 * actions and nothing to run them on.
 */

async function loaded(query = '') {
	const data = await load({
		url: new URL(`http://localhost/studio/posts/archive${query}`)
	} as never);
	return data as unknown as {
		posts: { id: string }[];
		total: number;
		page: number;
		perPage: number;
		failed: boolean;
	};
}

function backend(status: number, body: unknown) {
	fetchImpl.mockResolvedValue({ ok: status < 400, status, json: async () => body });
}

const PAGE = {
	data: { items: [{ id: 'post-1', title: 'Old' }], total: 12, page: 2, per_page: 10 }
};

beforeEach(() => {
	fetchImpl.mockReset();
	unreachable = null;
});

test('asks for archived posts, and only those', async () => {
	backend(200, PAGE);

	await loaded();

	const path = String(fetchImpl.mock.calls[0][1]);
	expect(path.startsWith('/api/blog?')).toBe(true);
	expect(path).toContain('deleted=true');
});

test('pages like every other list, from the URL', async () => {
	backend(200, PAGE);

	const data = await loaded('?page=2');

	expect(String(fetchImpl.mock.calls[0][1])).toContain('page=2');
	expect(String(fetchImpl.mock.calls[0][1])).toContain('per_page=10');
	expect(data).toMatchObject({ total: 12, page: 2, perPage: 10, failed: false });
	expect(data.posts).toHaveLength(1);
});

test('a failed fetch is a state, not an exception', async () => {
	backend(500, {});

	expect(await loaded()).toMatchObject({ failed: true, posts: [] });
});

test('an unreachable backend is the same state', async () => {
	unreachable = new TypeError('fetch failed');

	expect(await loaded()).toMatchObject({ failed: true, posts: [] });
});
