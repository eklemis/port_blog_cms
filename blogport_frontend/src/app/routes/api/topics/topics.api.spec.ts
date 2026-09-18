import { beforeEach, expect, test, vi } from 'vitest';

const fetchImpl = vi.fn();

/** See the note in the blog topics proxy spec. */
let unreachable: Error | null = null;

vi.mock('$lib/shared/api/backend.server', () => ({
	authenticatedFetch: async (...args: unknown[]) => {
		if (unreachable) throw unreachable;
		return fetchImpl(...args);
	}
}));

const { POST } = await import('./+server');

/**
 * `POST /api/topics` — the picker's inline create.
 *
 * §03: "Create topic (inline) · title, description · Create & attach". This is
 * the create; the attach is the editor's second request, and it needs the id
 * this hands back.
 */

const event = (body: unknown) => ({
	request: new Request('http://app.test', { method: 'POST', body: JSON.stringify(body) })
});

beforeEach(() => {
	unreachable = null;
	fetchImpl.mockReset();
});

test('hands back the topic it made, so it can be attached next', async () => {
	fetchImpl.mockResolvedValue({
		ok: true,
		status: 201,
		json: async () => ({ data: { id: 't-new', title: 'Postgres' } })
	});

	const response = await POST(event({ title: 'Postgres' }) as never);

	expect(await response.json()).toEqual({ data: { id: 't-new', title: 'Postgres' } });
	expect(fetchImpl.mock.calls[0][1]).toBe('/api/topics');
});

test('a refused title keeps its code, since the editor branches on it', async () => {
	fetchImpl.mockResolvedValue({
		ok: false,
		status: 409,
		json: async () => ({ error: { code: 'TOPIC_TITLE_TAKEN', message: 'x' } })
	});

	const response = await POST(event({ title: 'Rust' }) as never);

	expect(response.status).toBe(409);
	expect((await response.json()).error.code).toBe('TOPIC_TITLE_TAKEN');
});

test('an unreachable backend is a gateway failure', async () => {
	unreachable = new TypeError('Failed to fetch');

	expect((await POST(event({ title: 'Postgres' }) as never)).status).toBe(502);
});
