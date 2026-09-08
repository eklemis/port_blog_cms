import { beforeEach, expect, test, vi } from 'vitest';

const fetchImpl = vi.fn();

let unreachable: Error | null = null;

vi.mock('$lib/shared/api/backend.server', () => ({
	authenticatedFetch: async (...args: unknown[]) => {
		if (unreachable) throw unreachable;
		return fetchImpl(...args);
	}
}));

const { PATCH } = await import('./+server');

/**
 * `PATCH /api/blog/{id}` — the console's update proxy.
 *
 * Only the keys present in the body change, so the editor sends what moved and
 * never an object it did not load first.
 */

function event(body: unknown, id = 'post-1') {
	return {
		params: { id },
		request: new Request(`http://localhost/api/blog/${id}`, {
			method: 'PATCH',
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

test('sends only what it was given, to the post it names', async () => {
	backend(200, { data: { id: 'post-1', title: 'New title' } });

	await PATCH(event({ title: 'New title' }) as never);

	const [, path, init] = fetchImpl.mock.calls[0];
	expect(path).toBe('/api/blog/post-1');
	expect((init as RequestInit).method).toBe('PATCH');
	expect(JSON.parse((init as RequestInit).body as string)).toEqual({ title: 'New title' });
});

test('the id is escaped rather than pasted into a path', async () => {
	backend(200, { data: {} });

	await PATCH(event({ title: 'x' }, 'a/b?c') as never);

	expect(String(fetchImpl.mock.calls[0][1])).toBe('/api/blog/a%2Fb%3Fc');
});

test('a taken slug arrives as a code the editor can branch on', async () => {
	backend(409, { error: { code: 'SLUG_ALREADY_EXISTS', message: 'taken' } });

	const response = await PATCH(event({ slug: 'taken' }) as never);

	expect(response.status).toBe(409);
	expect(await response.json()).toMatchObject({ error: { code: 'SLUG_ALREADY_EXISTS' } });
});

test('an unreachable backend is a bad gateway, not an exception', async () => {
	unreachable = new TypeError('fetch failed');

	const response = await PATCH(event({ title: 'x' }) as never);

	expect(response.status).toBe(502);
	expect(await response.json()).toMatchObject({ error: { code: 'INTERNAL_ERROR' } });
});
