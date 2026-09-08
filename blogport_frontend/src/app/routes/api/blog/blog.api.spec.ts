import { beforeEach, expect, test, vi } from 'vitest';

const fetchImpl = vi.fn();

/** Set to make the call throw, without a rejecting mock Vitest would report. */
let unreachable: Error | null = null;

vi.mock('$lib/shared/api/backend.server', () => ({
	authenticatedFetch: async (...args: unknown[]) => {
		if (unreachable) throw unreachable;
		return fetchImpl(...args);
	}
}));

const { POST } = await import('./+server');

/**
 * `POST /api/blog` — the console's create proxy.
 *
 * The browser never holds a JWT, so a console mutation goes out through here
 * the same way a console list comes in. Like the auth proxies, it forwards the
 * error CODE and not only the prose: a slug collision is a branch that offers
 * a suggestion, and a sentence cannot be branched on.
 */

const DRAFT = { title: 'A post', slug: 'a-post', content: 'The first line.' };

function event(body: unknown = DRAFT) {
	return {
		request: new Request('http://localhost/api/blog', {
			method: 'POST',
			headers: { 'content-type': 'application/json' },
			body: JSON.stringify(body)
		})
	};
}

function backend(status: number, body: unknown, headers: Record<string, string> = {}) {
	fetchImpl.mockResolvedValue({
		ok: status < 400,
		status,
		headers: new Headers(headers),
		json: async () => body
	});
}

beforeEach(() => {
	fetchImpl.mockReset();
	unreachable = null;
});

test('sends the draft on, and answers with what came back', async () => {
	backend(201, { data: { id: 'post-1', slug: 'a-post' } });

	const response = await POST(event() as never);

	expect(response.status).toBe(201);
	expect(await response.json()).toMatchObject({ id: 'post-1' });

	const [, path, init] = fetchImpl.mock.calls[0];
	expect(path).toBe('/api/blog');
	expect(JSON.parse((init as RequestInit).body as string)).toEqual(DRAFT);
});

test('a taken slug arrives as a code, not as prose to parse', async () => {
	backend(409, { error: { code: 'SLUG_ALREADY_EXISTS', message: 'Slug already exists' } });

	const response = await POST(event() as never);

	expect(response.status).toBe(409);
	expect(await response.json()).toMatchObject({ error: { code: 'SLUG_ALREADY_EXISTS' } });
});

test('a rate limit keeps its Retry-After, so the countdown is the real one', async () => {
	backend(429, { error: { code: 'RATE_LIMITED' } }, { 'retry-after': '120' });

	const response = await POST(event() as never);

	expect(response.headers.get('retry-after')).toBe('120');
});

test('an unshaped error still has a code to handle', async () => {
	backend(500, 'not json at all');

	const response = await POST(event() as never);

	expect(response.status).toBe(500);
	expect(await response.json()).toMatchObject({ error: { code: 'INTERNAL_ERROR' } });
});

test('an unreachable backend is a bad gateway, not an exception', async () => {
	unreachable = new TypeError('fetch failed');

	const response = await POST(event() as never);

	expect(response.status).toBe(502);
	expect(await response.json()).toMatchObject({ error: { code: 'INTERNAL_ERROR' } });
});
