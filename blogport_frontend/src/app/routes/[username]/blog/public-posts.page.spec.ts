import { beforeEach, expect, test, vi } from 'vitest';

const fetchImpl = vi.fn();

/** Set to make the call throw. See the note in preview.page.spec.ts. */
let unreachable: Error | null = null;

vi.mock('$lib/shared/config/backend', () => ({
	backendBaseUrl: 'http://backend'
}));

const { load } = await import('./+page.server');

/**
 * `/[username]/blog` — an author's published posts, read by a stranger.
 *
 * Public and sessionless. The "Backed by" row names only the listing, but the
 * frame's own annotation at 72:86 says the header's name, bio and avatar come
 * from `GET /api/public/users/{username}` — a supporting call the route table
 * does not carry. That is the fourth screen where this has been true.
 */

const POSTS = {
	data: {
		total: 12,
		page: 1,
		per_page: 10,
		items: [
			{
				id: 'post-1',
				slug: 'building-a-cms-in-rust',
				title: 'Building a CMS in Rust',
				excerpt: 'A walk through the layout.',
				published_at: '2026-08-14T09:30:00Z',
				topics: [{ id: 't-rust', title: 'Rust' }]
			},
			{
				id: 'post-2',
				slug: 'notes-on-seaorm-migrations',
				title: 'Notes on SeaORM migrations',
				excerpt: 'Additive only.',
				published_at: '2026-08-02T09:30:00Z',
				topics: []
			}
		]
	}
};

const PROFILE = {
	data: {
		username: 'janedoe',
		full_name: 'Jane Doe',
		bio: 'Backend engineer, mostly Rust.',
		avatar: { media_id: 'av-1', variants: { thumbnail: '/api/public/media/av-1/thumbnail' } }
	}
};

function backend(
	answers: Partial<Record<'posts' | 'profile', { status: number; body?: unknown }>> = {}
) {
	const posts = answers.posts ?? { status: 200, body: POSTS };
	const profile = answers.profile ?? { status: 200, body: PROFILE };

	fetchImpl.mockImplementation(async (url: string) => {
		if (unreachable) throw unreachable;
		const answer = String(url).includes('/api/public/users/') ? profile : posts;
		return {
			ok: answer.status < 400,
			status: answer.status,
			json: async () => answer.body ?? null
		};
	});
}

const event = (username = 'janedoe', search = '') => ({
	params: { username },
	url: new URL(`http://app.test/${username}/blog${search}`),
	fetch: fetchImpl
});

beforeEach(() => {
	unreachable = null;
	fetchImpl.mockReset();
});

test('asks the public endpoints, with no session of any kind', async () => {
	backend();

	await load(event() as never);

	const urls = fetchImpl.mock.calls.map((c) => String(c[0]));
	expect(urls.some((u) => u.startsWith('http://backend/api/public/blog/janedoe?'))).toBe(true);
	expect(urls).toContain('http://backend/api/public/users/janedoe');
});

test('asks for a page, with the blueprint’s default size', async () => {
	backend();

	await load(event('janedoe', '?page=3') as never);

	const listing = fetchImpl.mock.calls
		.map((c) => String(c[0]))
		.find((u) => u.includes('/api/public/blog/janedoe?'));
	expect(listing).toContain('page=3');
	expect(listing).toContain('per_page=10');
});

test('passes a topic through to the listing rather than filtering in the page', async () => {
	// §03: both public listings accept topic_id. Filtering here would filter one
	// page and claim to have filtered the list.
	backend();

	await load(event('janedoe', '?topic_id=t-rust') as never);

	const listing = fetchImpl.mock.calls
		.map((c) => String(c[0]))
		.find((u) => u.includes('/api/public/blog/janedoe?'));
	expect(listing).toContain('topic_id=t-rust');
});

test('names the active filter from the posts it came back with', async () => {
	// The only public source for a topic's title: every listed post carries its
	// own topics, always present.
	backend();

	const data = (await load(event('janedoe', '?topic_id=t-rust') as never)) as {
		filter: { id: string; title: string } | null;
	};

	expect(data.filter).toEqual({ id: 't-rust', title: 'Rust' });
});

test('an unknown topic id is still a filter, even with nothing to name it', async () => {
	backend({
		posts: { status: 200, body: { data: { total: 0, page: 1, per_page: 10, items: [] } } }
	});

	const data = (await load(event('janedoe', '?topic_id=t-ghost') as never)) as {
		filter: { id: string; title: string } | null;
	};

	expect(data.filter).toEqual({ id: 't-ghost', title: 'this topic' });
});

test('spells each date the short way the list uses', async () => {
	backend();

	const data = (await load(event() as never)) as { posts: { published: string | null }[] };

	expect(data.posts[0].published).toContain('2026');
	expect(data.posts[0].published).not.toContain('August');
});

test('escapes what came out of the path', async () => {
	backend();

	await load(event('jane doe') as never);

	expect(String(fetchImpl.mock.calls[0][0])).toContain('/api/public/blog/jane%20doe');
});

test('an author nobody has is a plain 404 that names no username', async () => {
	// §03: "USER_NOT_FOUND 404 on the whole space. Never confirm which usernames
	// exist."
	backend({ profile: { status: 404, body: { error: { code: 'USER_NOT_FOUND' } } } });

	await expect(load(event() as never)).rejects.toMatchObject({ status: 404 });
});

test('a backend that broke is not an author who does not exist', async () => {
	// A 500 must not read as "no such person" — that is a different fact, and
	// saying it would be a lie about somebody's page.
	backend({ posts: { status: 500, body: { error: { code: 'INTERNAL_ERROR' } } } });

	await expect(load(event() as never)).rejects.toMatchObject({
		status: 500,
		body: { message: 'Something went wrong on our side.' }
	});
});

test('an unreachable backend is the same failure, not an empty list', async () => {
	unreachable = new Error('connect ECONNREFUSED');
	backend();

	await expect(load(event() as never)).rejects.toMatchObject({ status: 500 });
});
