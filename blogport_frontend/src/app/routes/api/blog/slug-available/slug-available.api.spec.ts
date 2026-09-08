import { beforeEach, expect, test, vi } from 'vitest';

const fetchImpl = vi.fn();

let unreachable: Error | null = null;

vi.mock('$lib/shared/api/backend.server', () => ({
	authenticatedFetch: async (...args: unknown[]) => {
		if (unreachable) throw unreachable;
		return fetchImpl(...args);
	}
}));

const { GET } = await import('./+server');

/**
 * `GET /api/blog/slug-available`.
 *
 * The backend answers with a free variant when one is taken, so the "-2"
 * suggestion J4 asks for is the API's answer rather than something invented
 * here — and it is the address that is actually free, which a guess is not.
 */

const event = (slug = 'a-post') => ({
	url: new URL(`http://localhost/api/blog/slug-available?slug=${encodeURIComponent(slug)}`)
});

function backend(status: number, body: unknown) {
	fetchImpl.mockResolvedValue({
		ok: status < 400,
		status,
		headers: new Headers(),
		json: async () => body
	});
}

beforeEach(() => {
	fetchImpl.mockReset();
	unreachable = null;
});

test('passes the candidate through, escaped', async () => {
	backend(200, { data: { available: true, slug: 'a post', suggestion: null } });

	await GET(event('a post & more') as never);

	expect(String(fetchImpl.mock.calls[0][1])).toBe('/api/blog/slug-available?slug=a+post+%26+more');
});

test('hands back availability and the free variant', async () => {
	backend(200, {
		data: { available: false, slug: 'a-post', suggestion: 'a-post-2' }
	});

	const response = await GET(event() as never);

	expect(await response.json()).toEqual({
		available: false,
		slug: 'a-post',
		suggestion: 'a-post-2'
	});
});

test('an empty candidate is not asked about at all', async () => {
	// There is nothing to check, and the backend would answer about "".
	const response = await GET(event('  ') as never);

	expect(fetchImpl).not.toHaveBeenCalled();
	expect(await response.json()).toEqual({ available: false, slug: '', suggestion: null });
});

test('a check that fails says nothing rather than blocking the form', async () => {
	// This is a courtesy: the real answer comes from the create call, which
	// returns SLUG_ALREADY_EXISTS. A failed check must not look like a taken
	// slug or the person cannot submit at all.
	backend(500, { error: { code: 'INTERNAL_ERROR' } });

	const response = await GET(event() as never);

	expect(response.status).toBe(200);
	expect(await response.json()).toEqual({ available: true, slug: 'a-post', suggestion: null });
});

test('an unreachable backend is the same courtesy', async () => {
	unreachable = new TypeError('fetch failed');

	const response = await GET(event() as never);

	expect(await response.json()).toMatchObject({ available: true });
});
