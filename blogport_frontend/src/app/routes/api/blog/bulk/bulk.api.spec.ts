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
 * `POST /api/blog/bulk`. A 200 means the batch ran, not that every item did —
 * so the outcome is forwarded whole, `failed` included, and never flattened
 * into a yes or a no.
 */

const event = (body: unknown) => ({
	request: new Request('http://localhost/api/blog/bulk', {
		method: 'POST',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify(body)
	})
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

test('sends the operation and the ids on, as they came', async () => {
	backend(200, { data: { succeeded: ['a'], failed: [] } });

	await POST(event({ op: 'restore', ids: ['a', 'b'] }) as never);

	const [, path, init] = fetchImpl.mock.calls[0];
	expect(path).toBe('/api/blog/bulk');
	expect(JSON.parse((init as RequestInit).body as string)).toEqual({
		op: 'restore',
		ids: ['a', 'b']
	});
});

test('a partial batch comes back whole, failures included', async () => {
	const outcome = {
		succeeded: ['a'],
		failed: [{ id: 'b', code: 'POST_NOT_FOUND', message: 'Blog post not found' }]
	};
	backend(200, { data: outcome });

	const response = await POST(event({ op: 'hard_delete', ids: ['a', 'b'] }) as never);

	expect(response.status).toBe(200);
	expect(await response.json()).toEqual(outcome);
});

test('a refused batch arrives as a code', async () => {
	backend(400, { error: { code: 'BULK_TOO_LARGE', message: 'too many' } });

	const response = await POST(event({ op: 'restore', ids: [] }) as never);

	expect(response.status).toBe(400);
	expect(await response.json()).toMatchObject({ error: { code: 'BULK_TOO_LARGE' } });
});

test('an unreachable backend is a bad gateway, not an exception', async () => {
	unreachable = new TypeError('fetch failed');

	expect((await POST(event({ op: 'restore', ids: ['a'] }) as never)).status).toBe(502);
});
