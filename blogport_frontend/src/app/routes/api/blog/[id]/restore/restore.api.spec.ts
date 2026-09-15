import { beforeEach, expect, test, vi } from 'vitest';

const fetchImpl = vi.fn();
let unreachable: Error | null = null;

vi.mock('$lib/shared/api/backend.server', () => ({
	authenticatedFetch: async (...args: unknown[]) => {
		if (unreachable) throw unreachable;
		return fetchImpl(...args);
	}
}));

const { POST } = await import('./+server');

/** `POST /api/blog/{id}/restore` — the second rung, back out of the archive. */

const event = (id = 'post-1') => ({ params: { id } });

function backend(status: number, body: unknown = null) {
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

test('restores the post it names, escaped', async () => {
	backend(204);

	const response = await POST(event('a/b') as never);

	expect(response.status).toBe(204);
	const [, path, init] = fetchImpl.mock.calls[0];
	expect(path).toBe('/api/blog/a%2Fb/restore');
	expect((init as RequestInit).method).toBe('POST');
});

test('a post that is not archived arrives as a code, not prose', async () => {
	backend(404, { error: { code: 'POST_NOT_FOUND', message: 'not found' } });

	const response = await POST(event() as never);

	expect(response.status).toBe(404);
	expect(await response.json()).toMatchObject({ error: { code: 'POST_NOT_FOUND' } });
});

test('an unreachable backend is a bad gateway, not an exception', async () => {
	unreachable = new TypeError('fetch failed');

	expect((await POST(event() as never)).status).toBe(502);
});
