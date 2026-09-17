import { beforeEach, expect, test, vi } from 'vitest';

const fetchImpl = vi.fn();

/**
 * Set to make the call throw. A plain variable rather than a rejecting mock:
 * Vitest tracks a `vi.fn`'s settled result too, and reports that second promise
 * as unhandled even when the loader catches the first.
 */
let unreachable: Error | null = null;

// The module the loader actually imports. Mocking `api/backend.server` here
// named a module this route never loads, so it replaced nothing.
vi.mock('$lib/shared/config/backend', () => ({
	backendBaseUrl: 'http://backend'
}));

const { load } = await import('./+page.server');

/**
 * `/preview/[token]` — a draft read through its share link.
 *
 * Public: the token is the authorisation, so this load carries no session and
 * uses the public endpoint. A dead, revoked or unknown token is a 404 and says
 * so; a post that has since been published is not a draft any more and the
 * reader is sent to the real page.
 */

function backend(status: number, body: unknown = null, headers: Record<string, string> = {}) {
	fetchImpl.mockImplementation(async () => {
		if (unreachable) throw unreachable;
		return {
			ok: status < 400,
			status,
			headers: new Headers(headers),
			json: async () => body
		};
	});
}

const event = (token = 'tok-1') => ({ params: { token }, fetch: fetchImpl });

const POST = {
	id: 'post-1',
	title: 'Building a CMS in Rust',
	slug: 'building-a-cms-in-rust',
	content: 'The API is one Actix Web service.',
	excerpt: 'A walk through the layout.',
	published_at: null,
	topics: [{ id: 't1', title: 'Rust' }],
	preview: true
};

beforeEach(() => {
	fetchImpl.mockReset();
	unreachable = null;
});

test('reads the draft through the token, with no session', async () => {
	backend(200, { data: POST });

	const data = (await load(event() as never)) as unknown as { post: typeof POST };

	expect(String(fetchImpl.mock.calls[0][0])).toContain('/api/public/blog/preview/tok-1');
	expect(data.post.title).toBe('Building a CMS in Rust');
});

test('the token is escaped rather than pasted into a path', async () => {
	backend(200, { data: POST });

	await load(event('a/b') as never);

	expect(String(fetchImpl.mock.calls[0][0])).toContain('/api/public/blog/preview/a%2Fb');
});

test('a dead link is a 404, not a blank page', async () => {
	backend(404, { error: { code: 'POST_NOT_FOUND' } });

	await expect(load(event() as never)).rejects.toMatchObject({ status: 404 });
});

test('an unreachable backend is a 404 too, since nothing can be shown', async () => {
	unreachable = new TypeError('fetch failed');
	backend(200, { data: POST });

	await expect(load(event() as never)).rejects.toMatchObject({ status: 404 });
});
