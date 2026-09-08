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
 * The editor's loader.
 *
 * `GET /api/blog/{id}` returns drafts as well as published posts, with their
 * topics — and reports someone else's post as not found rather than forbidden,
 * which is why both outcomes land on the same screen.
 */

const POST = {
	id: 'post-1',
	title: 'Building a CMS',
	slug: 'building-a-cms',
	content: 'The first line.',
	published_at: null,
	topics: []
};

function backend(status: number, body: unknown) {
	fetchImpl.mockResolvedValue({
		ok: status < 400,
		status,
		headers: new Headers(),
		json: async () => body
	});
}

async function loaded(id = 'post-1') {
	const data = await load({ params: { id } } as never);
	return data as unknown as { post: typeof POST | null; denied: boolean };
}

beforeEach(() => {
	fetchImpl.mockReset();
	unreachable = null;
});

test('asks for the post it was routed to', async () => {
	backend(200, { data: POST });

	await loaded();

	expect(String(fetchImpl.mock.calls[0][1])).toBe('/api/blog/post-1');
});

test('the id is escaped rather than pasted into a path', async () => {
	backend(200, { data: POST });

	await loaded('a/b');

	expect(String(fetchImpl.mock.calls[0][1])).toBe('/api/blog/a%2Fb');
});

test('hands the post through', async () => {
	backend(200, { data: POST });

	const data = await loaded();

	expect(data.post).toMatchObject({ id: 'post-1', title: 'Building a CMS' });
	expect(data.denied).toBe(false);
});

test('a post that is not yours is refused, not crashed on', async () => {
	// The API reports someone else's post as not found; J4 calls the same
	// situation POST_UNAUTHORIZED. Both mean one screen — see the PR.
	backend(404, { error: { code: 'POST_NOT_FOUND' } });

	const data = await loaded();

	expect(data).toEqual({ post: null, denied: true });
});

test('a forbidden post is the same screen', async () => {
	backend(403, { error: { code: 'POST_UNAUTHORIZED' } });

	expect(await loaded()).toEqual({ post: null, denied: true });
});

test('an unreachable backend does not throw out of the loader', async () => {
	unreachable = new TypeError('fetch failed');

	expect(await loaded()).toEqual({ post: null, denied: true });
});
