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
 * `POST /api/cvs` — the list's "+ New résumé".
 *
 * It exists because the browser cannot reach the API directly: the access token
 * is in an httpOnly cookie, so every write from a component goes through a
 * route like this one. The create call was written against `/api/cvs` and this
 * route was not there — the spec mocked `fetch`, which proved the request was
 * well-formed and nothing at all about there being anything to receive it.
 */

const event = (body: unknown) => ({
	request: new Request('http://app.test', { method: 'POST', body: JSON.stringify(body) })
});

beforeEach(() => {
	unreachable = null;
	fetchImpl.mockReset();
});

test('hands back the id the builder is opened at', async () => {
	fetchImpl.mockResolvedValue({
		ok: true,
		status: 201,
		json: async () => ({ data: { id: 'cv-new', role: '' } })
	});

	const response = await POST(event({ display_name: 'Ada' }) as never);

	expect(response.status).toBe(201);
	expect(await response.json()).toEqual({ data: { id: 'cv-new', role: '' } });
});

test('relays the body as written, deciding nothing about what a résumé holds', async () => {
	fetchImpl.mockResolvedValue({ ok: true, status: 201, json: async () => ({ data: { id: 'x' } }) });

	await POST(event({ display_name: 'Ada', role: '', experiences: [] }) as never);

	// The event is the first argument; the path and the options follow it.
	const [, path, init] = fetchImpl.mock.calls[0] as [unknown, string, RequestInit];
	expect(path).toBe('/api/cvs');
	expect(init.method).toBe('POST');
	expect(JSON.parse(String(init.body))).toEqual({
		display_name: 'Ada',
		role: '',
		experiences: []
	});
});

test('a refusal keeps its code, because the code picks the screen', async () => {
	fetchImpl.mockResolvedValue({
		ok: false,
		status: 403,
		json: async () => ({ error: { code: 'EMAIL_NOT_VERIFIED', message: 'Verify first' } })
	});

	const response = await POST(event({}) as never);

	expect(response.status).toBe(403);
	expect(await response.json()).toEqual({
		error: { code: 'EMAIL_NOT_VERIFIED', message: 'Verify first' }
	});
});

test('an unshaped failure still arrives shaped', async () => {
	fetchImpl.mockResolvedValue({ ok: false, status: 500, json: async () => 'gateway said no' });

	const response = await POST(event({}) as never);
	const body = (await response.json()) as { error: { code: string } };

	expect(body.error.code).toBe('INTERNAL_ERROR');
});

test('an unreachable API is 502, not an unhandled throw', async () => {
	unreachable = new TypeError('connect ECONNREFUSED');

	const response = await POST(event({}) as never);

	expect(response.status).toBe(502);
});
