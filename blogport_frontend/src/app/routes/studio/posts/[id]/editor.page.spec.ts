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

	expect(data).toEqual({
		post: null,
		denied: true,
		availableTopics: [],
		cover: null,
		coverSrc: null
	});
});

test('a forbidden post is the same screen', async () => {
	backend(403, { error: { code: 'POST_UNAUTHORIZED' } });

	expect(await loaded()).toEqual({
		post: null,
		denied: true,
		availableTopics: [],
		cover: null,
		coverSrc: null
	});
});

test('an unreachable backend does not throw out of the loader', async () => {
	unreachable = new TypeError('fetch failed');

	expect(await loaded()).toEqual({
		post: null,
		denied: true,
		availableTopics: [],
		cover: null,
		coverSrc: null
	});
});

test('brings the author’s own topics, for the rail to offer', async () => {
	// §03: "own topics only". The picker offers what this person has, so the
	// list comes with the page rather than on first click — one fewer thing to
	// wait for, and the editor is useless without the post anyway.
	fetchImpl.mockImplementation(async (_event: unknown, path: string) => ({
		ok: true,
		status: 200,
		headers: new Headers(),
		json: async () =>
			String(path).startsWith('/api/topics')
				? { data: { items: [{ id: 't-1', title: 'Rust' }] } }
				: { data: POST }
	}));

	const data = (await load({ params: { id: 'post-1' } } as never)) as {
		availableTopics: { id: string; title: string }[];
	};

	expect(data.availableTopics).toEqual([{ id: 't-1', title: 'Rust' }]);
});

test('a topics list that could not be had is no topics, not a broken editor', async () => {
	// The post is the page. Losing the pick-list costs the picker its options
	// and nothing else, so it must not take the screen down with it.
	fetchImpl.mockImplementation(async (_event: unknown, path: string) =>
		String(path).startsWith('/api/topics')
			? { ok: false, status: 500, headers: new Headers(), json: async () => ({}) }
			: { ok: true, status: 200, headers: new Headers(), json: async () => ({ data: POST }) }
	);

	const data = (await load({ params: { id: 'post-1' } } as never)) as {
		post: unknown;
		availableTopics: unknown[];
	};

	expect(data.post).toBeTruthy();
	expect(data.availableTopics).toEqual([]);
});

/**
 * The cover. Two calls, because neither endpoint can answer alone:
 * `by-target/blog_post` lists every image on every post this author owns, and
 * the read URL is a second request that only makes sense once one is `ready`.
 */

/** Answers each backend path in turn, so the two cover calls can differ. */
function routes(answers: Record<string, { status?: number; body: unknown }>) {
	fetchImpl.mockImplementation(async (_event: unknown, path: string) => {
		const answer = answers[path] ?? { status: 404, body: { error: { code: 'NOT_FOUND' } } };
		const status = answer.status ?? 200;

		return { ok: status < 400, status, headers: new Headers(), json: async () => answer.body };
	});
}

const attachment = (over: Record<string, unknown> = {}) => ({
	media_id: 'm-1',
	attachment_target_id: 'post-1',
	role: 'cover',
	status: 'ready',
	alt_text: 'A hexagonal diagram',
	...over
});

test('a ready cover arrives with a URL to read it with', async () => {
	routes({
		'/api/blog/post-1': { body: { data: POST } },
		'/api/topics': { body: { data: [] } },
		'/api/media/by-target/blog_post?target_id=post-1&role=cover': {
			body: { data: { rows: [attachment()] } }
		},
		'/api/media/m-1/medium': { body: { data: { url: 'https://signed.example.test/m-1' } } }
	});

	const data = (await load({ params: { id: 'post-1' } } as never)) as unknown as {
		cover: { media_id: string } | null;
		coverSrc: string | null;
	};

	expect(data.cover?.media_id).toBe('m-1');
	expect(data.coverSrc).toBe('https://signed.example.test/m-1');
});

test('a cover still processing is passed on without a URL it does not have', async () => {
	// There are no variants to read yet. Asking would answer 409, and the card
	// shows the state rather than a gap.
	routes({
		'/api/blog/post-1': { body: { data: POST } },
		'/api/topics': { body: { data: [] } },
		'/api/media/by-target/blog_post?target_id=post-1&role=cover': {
			body: { data: { rows: [attachment({ status: 'processing' })] } }
		}
	});

	const data = (await load({ params: { id: 'post-1' } } as never)) as unknown as {
		cover: { status: string } | null;
		coverSrc: string | null;
	};

	expect(data.cover?.status).toBe('processing');
	expect(data.coverSrc).toBe(null);
	expect(fetchImpl.mock.calls.map((call) => call[1])).not.toContain('/api/media/m-1/medium');
});

test('another post’s images are not this post’s cover', async () => {
	// The listing is per target *kind*, not per post.
	routes({
		'/api/blog/post-1': { body: { data: POST } },
		'/api/topics': { body: { data: [] } },
		'/api/media/by-target/blog_post?target_id=post-1&role=cover': {
			body: { data: { rows: [attachment({ attachment_target_id: 'post-2' })] } }
		}
	});

	const data = (await load({ params: { id: 'post-1' } } as never)) as unknown as {
		cover: unknown | null;
	};

	expect(data.cover).toBe(null);
});

test('losing the media listing costs the rail its picture and nothing else', async () => {
	routes({
		'/api/blog/post-1': { body: { data: POST } },
		'/api/topics': { body: { data: [] } }
	});

	const data = (await load({ params: { id: 'post-1' } } as never)) as unknown as {
		post: { id: string } | null;
		cover: unknown | null;
	};

	expect(data.post?.id).toBe('post-1');
	expect(data.cover).toBe(null);
});

test('asks for this post’s cover rather than the author’s whole library', async () => {
	// `target_id` and `role` shipped as "Let a caller ask for one thing's
	// media". Before them this fetched every image on every post ever written
	// and filtered client-side.
	routes({
		'/api/blog/post-1': { body: { data: POST } },
		'/api/topics': { body: { data: [] } },
		'/api/media/by-target/blog_post?target_id=post-1&role=cover': {
			body: { data: { rows: [] } }
		}
	});

	await load({ params: { id: 'post-1' } } as never);

	const asked = fetchImpl.mock.calls.map((call) => String(call[1]));

	expect(asked).toContain('/api/media/by-target/blog_post?target_id=post-1&role=cover');
	expect(asked).not.toContain('/api/media/by-target/blog_post');
});
