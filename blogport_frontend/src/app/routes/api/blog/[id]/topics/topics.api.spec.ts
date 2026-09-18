import { beforeEach, expect, test, vi } from 'vitest';

const fetchImpl = vi.fn();

/**
 * Set to make the call throw. A plain variable rather than a rejecting mock:
 * Vitest tracks a `vi.fn`'s settled result too, and reports that rejection as
 * unhandled even once the handler has caught it.
 */
let unreachable: Error | null = null;

vi.mock('$lib/shared/api/backend.server', () => ({
	authenticatedFetch: async (...args: unknown[]) => {
		if (unreachable) throw unreachable;
		return fetchImpl(...args);
	}
}));

const { POST, DELETE } = await import('./+server');

/**
 * `POST`/`DELETE /api/blog/{id}/topics` — one chip on, one chip off.
 *
 * The browser cannot call the backend directly: it holds no token, and the
 * session cookie is this app's. Without these the picker's requests land on
 * SvelteKit and 404 — which unit tests injecting a `fetchFn` never notice.
 */

const event = (body: unknown, id = 'post-1') => ({
	params: { id },
	request: new Request('http://app.test', { method: 'POST', body: JSON.stringify(body) })
});

beforeEach(() => {
	unreachable = null;
	fetchImpl.mockReset();
});

test('attaching passes the topic through and answers with no content', async () => {
	fetchImpl.mockResolvedValue({ ok: true, status: 204, json: async () => null });

	const response = await POST(event({ topic_id: 't-1' }) as never);

	expect(response.status).toBe(204);
	expect(fetchImpl.mock.calls[0][1]).toBe('/api/blog/post-1/topics');
	expect(fetchImpl.mock.calls[0][2]).toMatchObject({ method: 'POST' });
});

test('detaching uses the same address with the other verb', async () => {
	fetchImpl.mockResolvedValue({ ok: true, status: 204, json: async () => null });

	await DELETE(event({ topic_id: 't-1' }) as never);

	expect(fetchImpl.mock.calls[0][2]).toMatchObject({ method: 'DELETE' });
});

test('escapes the id rather than pasting it into the path', async () => {
	fetchImpl.mockResolvedValue({ ok: true, status: 204, json: async () => null });

	await POST(event({ topic_id: 't-1' }, 'a/b') as never);

	expect(fetchImpl.mock.calls[0][1]).toBe('/api/blog/a%2Fb/topics');
});

test('a refusal keeps its own code, so the editor can classify it', async () => {
	fetchImpl.mockResolvedValue({
		ok: false,
		status: 409,
		json: async () => ({ error: { code: 'TOPIC_ALREADY_ATTACHED', message: 'x' } })
	});

	const response = await POST(event({ topic_id: 't-1' }) as never);

	expect(response.status).toBe(409);
	expect((await response.json()).error.code).toBe('TOPIC_ALREADY_ATTACHED');
});

test('an unreachable backend is a gateway failure, not a success', async () => {
	unreachable = new TypeError('Failed to fetch');

	expect((await POST(event({ topic_id: 't-1' }) as never)).status).toBe(502);
});
