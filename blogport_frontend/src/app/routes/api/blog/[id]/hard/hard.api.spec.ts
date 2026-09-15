import { beforeEach, expect, test, vi } from 'vitest';

const fetchImpl = vi.fn();
let unreachable: Error | null = null;

vi.mock('$lib/shared/api/backend.server', () => ({
	authenticatedFetch: async (...args: unknown[]) => {
		if (unreachable) throw unreachable;
		return fetchImpl(...args);
	}
}));

const { DELETE } = await import('./+server');

/** `DELETE /api/blog/{id}/hard` — the third rung, and the only one with no way back. */

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

test('deletes the post it names, escaped', async () => {
	backend(204);

	const response = await DELETE(event('a/b') as never);

	expect(response.status).toBe(204);
	const [, path, init] = fetchImpl.mock.calls[0];
	expect(path).toBe('/api/blog/a%2Fb/hard');
	expect((init as RequestInit).method).toBe('DELETE');
});

test('a post that is gone arrives as a code, not prose', async () => {
	backend(404, { error: { code: 'POST_NOT_FOUND', message: 'not found' } });

	const response = await DELETE(event() as never);

	expect(response.status).toBe(404);
	expect(await response.json()).toMatchObject({ error: { code: 'POST_NOT_FOUND' } });
});

test('an unreachable backend is a bad gateway, not an exception', async () => {
	unreachable = new TypeError('fetch failed');

	expect((await DELETE(event() as never)).status).toBe(502);
});
