import { beforeEach, expect, test, vi } from 'vitest';

const fetchImpl = vi.fn();

/** Set to make the call throw. See the note in preview.page.spec.ts. */
let unreachable: Error | null = null;

vi.mock('$lib/shared/config/backend', () => ({
	backendBaseUrl: 'http://backend'
}));

const { load } = await import('./+page.server');

/**
 * `/[username]/blog/[slug]` — a published post, read by a stranger.
 *
 * Public and sessionless. §03: every public route is server-rendered "so a
 * shared link is a real page with real metadata — not a shell that fetches
 * after paint", which is also why Markdown becomes HTML here and not in the
 * browser.
 */

const POST = {
	id: 'post-1',
	title: 'Building a CMS in Rust',
	slug: 'building-a-cms-in-rust',
	content: '## Layout\n\nThe API is one Actix Web service.',
	excerpt: 'A walk through the layout.',
	published_at: '2026-08-14T09:30:00Z',
	topics: [{ id: 't1', title: 'Rust' }],
	cover: {
		media_id: 'cov-1',
		alt_text: 'Hexagonal layout',
		caption: '',
		position: 0,
		role: 'cover',
		variants: { large: '/api/public/media/cov-1/large' }
	}
};

const PROFILE = {
	username: 'janedoe',
	full_name: 'Jane Doe',
	bio: 'Backend engineer, mostly Rust.',
	avatar: { media_id: 'av-1', variants: { thumbnail: '/api/public/media/av-1/thumbnail' } }
};

/** The post call and the profile call, answered by URL. */
function backend(
	answers: Partial<Record<'post' | 'profile', { status: number; body?: unknown }>> = {}
) {
	const post = answers.post ?? { status: 200, body: { data: POST } };
	const profile = answers.profile ?? { status: 200, body: { data: PROFILE } };

	fetchImpl.mockImplementation(async (url: string) => {
		if (unreachable) throw unreachable;
		const answer = String(url).includes('/api/public/users/') ? profile : post;
		return {
			ok: answer.status < 400,
			status: answer.status,
			json: async () => answer.body ?? null
		};
	});
}

const event = (username = 'janedoe', slug = 'building-a-cms-in-rust') => ({
	params: { username, slug },
	fetch: fetchImpl,
	setHeaders: vi.fn()
});

beforeEach(() => {
	unreachable = null;
	fetchImpl.mockReset();
});

test('asks the public endpoints, with no session of any kind', async () => {
	backend();

	await load(event() as never);

	const urls = fetchImpl.mock.calls.map((c) => String(c[0]));
	expect(urls).toContain('http://backend/api/public/blog/janedoe/building-a-cms-in-rust');
	expect(urls).toContain('http://backend/api/public/users/janedoe');
});

test('escapes what came out of the path rather than pasting it into a URL', async () => {
	backend();

	await load(event('jane doe', 'a/b') as never);

	expect(String(fetchImpl.mock.calls[0][0])).toContain('/api/public/blog/jane%20doe/a%2Fb');
});

test('hands the page finished markup, not the source', async () => {
	// The frame's own note: "Markdown → HTML happens in +page.server.ts; no
	// parser ships to the reader."
	backend();

	const data = (await load(event() as never)) as { bodyHtml: string };

	expect(data.bodyHtml).toContain('<h2>Layout</h2>');
	expect(data.bodyHtml).not.toContain('## Layout');
});

test('points the cover at the backend, since a relative path would hit this app', async () => {
	backend();

	const data = (await load(event() as never)) as { cover: { src: string; alt: string } };

	expect(data.cover).toEqual({
		src: 'http://backend/api/public/media/cov-1/large',
		alt: 'Hexagonal layout'
	});
});

test('a cover whose sizes are still generating is no cover at all', async () => {
	// "variants can be empty while sizes are still generating" — a known state,
	// not a failure.
	backend({
		post: { status: 200, body: { data: { ...POST, cover: { ...POST.cover, variants: {} } } } }
	});

	const data = (await load(event() as never)) as { cover: unknown };

	expect(data.cover).toBe(null);
});

test('counts the reading time from the body it was given', async () => {
	backend();

	const data = (await load(event() as never)) as { readMinutes: number };

	expect(data.readMinutes).toBe(1);
});

test('a post that does not exist says so in the words the blueprint chose', async () => {
	// Also what an unpublished, scheduled or archived post answers — the same
	// 404 on purpose, so the route cannot be used to find out which posts exist.
	backend({ post: { status: 404, body: { error: { code: 'POST_NOT_FOUND' } } } });

	await expect(load(event() as never)).rejects.toMatchObject({
		status: 404,
		body: { message: "That post doesn't exist, or it isn't published yet." }
	});
});

test('an unknown author never gets confirmed as unknown', async () => {
	// §03: "USER_NOT_FOUND 404 on the whole space. Never confirm which usernames
	// exist." So the answer is the same one a missing post gets.
	backend({ profile: { status: 404, body: { error: { code: 'USER_NOT_FOUND' } } } });

	await expect(load(event() as never)).rejects.toMatchObject({ status: 404 });
});

test('an unreachable backend is a 404 too, because nothing can be shown', async () => {
	unreachable = new Error('connect ECONNREFUSED');
	backend();

	await expect(load(event() as never)).rejects.toMatchObject({ status: 404 });
});
