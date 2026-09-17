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

/**
 * `POST /api/blog/{id}/preview` — the share link behind the editor's Preview.
 *
 * It creates the link or extends the one that exists, so pressing Preview
 * twice is not two links to keep track of.
 */

const event = (id = 'post-1') => ({ params: { id } });

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

test('asks for the post’s link and hands back the token', async () => {
	backend(200, { data: { token: 'tok-1', expires_at: '2026-09-24T12:00:00Z', expired: false } });

	const response = await POST(event() as never);

	expect(String(fetchImpl.mock.calls[0][1])).toBe('/api/blog/post-1/preview');
	expect((fetchImpl.mock.calls[0][2] as RequestInit).method).toBe('POST');
	expect(await response.json()).toMatchObject({ token: 'tok-1' });
});

test('a post that is gone arrives as a code', async () => {
	backend(403, { error: { code: 'POST_UNAUTHORIZED' } });

	const response = await POST(event() as never);

	expect(response.status).toBe(403);
	expect(await response.json()).toMatchObject({ error: { code: 'POST_UNAUTHORIZED' } });
});

test('an unreachable backend is a bad gateway, not an exception', async () => {
	unreachable = new TypeError('fetch failed');

	expect((await POST(event() as never)).status).toBe(502);
});
