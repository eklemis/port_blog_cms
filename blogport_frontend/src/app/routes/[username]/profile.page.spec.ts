import { beforeEach, expect, test, vi } from 'vitest';

const fetchImpl = vi.fn();

let unreachable: Error | null = null;

vi.mock('$lib/shared/config/backend', () => ({ backendBaseUrl: 'http://backend' }));

const { load } = await import('./+page.server');

/**
 * `/[username]` — the front door.
 *
 * §03's row names all three calls: the profile, and the two listings behind
 * the "All →" strips. The profile is the page; the strips are not, and they
 * fail on their own without taking it down.
 */

const PROFILE = {
	data: { username: 'janedoe', full_name: 'Jane Doe', bio: 'Mostly Rust.', avatar: null }
};

const POSTS = {
	data: {
		total: 12,
		items: [
			{ slug: 'a', title: 'A', published_at: '2026-08-14T00:00:00Z', topics: [] },
			{ slug: 'b', title: 'B', published_at: '2026-08-02T00:00:00Z', topics: [] },
			{ slug: 'c', title: 'C', published_at: '2026-07-21T00:00:00Z', topics: [] },
			{ slug: 'd', title: 'D', published_at: '2026-07-01T00:00:00Z', topics: [] }
		]
	}
};

const PROJECTS = {
	data: {
		total: 8,
		items: [1, 2, 3, 4].map((n) => ({
			slug: `p${n}`,
			title: `P${n}`,
			description: 'x',
			tech_stack: ['Rust'],
			topics: [],
			cover: null
		}))
	}
};

function backend(
	over: Partial<Record<'profile' | 'posts' | 'projects', { status: number; body?: unknown }>> = {}
) {
	const answers = {
		profile: over.profile ?? { status: 200, body: PROFILE },
		posts: over.posts ?? { status: 200, body: POSTS },
		projects: over.projects ?? { status: 200, body: PROJECTS }
	};

	fetchImpl.mockImplementation(async (url: string) => {
		if (unreachable) throw unreachable;
		const u = String(url);
		const a = u.includes('/api/public/users/')
			? answers.profile
			: u.includes('/api/public/projects/')
				? answers.projects
				: answers.posts;
		return { ok: a.status < 400, status: a.status, json: async () => a.body ?? null };
	});
}

const event = (username = 'janedoe') => ({ params: { username }, fetch: fetchImpl });

beforeEach(() => {
	unreachable = null;
	fetchImpl.mockReset();
});

test('asks for the profile and both strips', async () => {
	backend();

	await load(event() as never);

	const urls = fetchImpl.mock.calls.map((c) => String(c[0]));
	expect(urls).toContain('http://backend/api/public/users/janedoe');
	expect(urls.some((u) => u.includes('/api/public/blog/janedoe?'))).toBe(true);
	expect(urls.some((u) => u.includes('/api/public/projects/janedoe?'))).toBe(true);
});

test('takes a few of each, not a page of each', async () => {
	// It is a doorway. Asking for ten and drawing three wastes the other seven.
	backend();

	const data = (await load(event() as never)) as { posts: unknown[]; projects: unknown[] };

	expect(data.posts).toHaveLength(3);
	expect(data.projects).toHaveLength(3);
});

test('a strip that could not be had is no strip, not a broken page', async () => {
	// The profile is the page; the strips are what it points at. One failing is
	// a door fewer, and the other door still works.
	backend({ projects: { status: 500, body: null } });

	const data = (await load(event() as never)) as { projects: unknown[]; posts: unknown[] };

	expect(data.projects).toEqual([]);
	expect(data.posts).toHaveLength(3);
});

test('an author nobody has is a plain 404', async () => {
	backend({ profile: { status: 404, body: { error: { code: 'USER_NOT_FOUND' } } } });

	await expect(load(event() as never)).rejects.toMatchObject({ status: 404 });
});

test('a profile that broke is not an author who does not exist', async () => {
	backend({ profile: { status: 500, body: null } });

	await expect(load(event() as never)).rejects.toMatchObject({ status: 500 });
});

test('escapes what came out of the path', async () => {
	backend();

	await load(event('jane doe') as never);

	expect(String(fetchImpl.mock.calls[0][0])).toContain('jane%20doe');
});
