import { beforeEach, expect, test, vi } from 'vitest';

const fetchImpl = vi.fn();

vi.mock('$lib/shared/api/backend.server', () => ({
	authenticatedFetch: async (...args: unknown[]) => fetchImpl(...args)
}));

const { POST } = await import('./+server');

/**
 * `POST /api/locale` — where the switcher writes.
 *
 * §02: the cookie first, because the auth and public shells have no session.
 * When there *is* one, the account is told too, so the choice survives the
 * browser it was made in.
 */

const event = (locale: unknown, user: unknown = null) => ({
	request: new Request('http://app.test', { method: 'POST', body: JSON.stringify({ locale }) }),
	cookies: { set: vi.fn(), get: vi.fn() },
	locals: { user }
});

beforeEach(() => fetchImpl.mockReset());

test('writes the cookie, so a shell with no session still remembers', async () => {
	const e = event('en');

	const response = await POST(e as never);

	expect(response.status).toBe(204);
	expect(e.cookies.set).toHaveBeenCalledWith(
		'arch_locale',
		'en',
		expect.objectContaining({ path: '/' })
	);
});

test('tells the account as well, when there is one', async () => {
	fetchImpl.mockResolvedValue({ ok: true, status: 200, json: async () => ({}) });
	const e = event('en', { user_id: 'u-1' });

	await POST(e as never);

	expect(fetchImpl.mock.calls[0][1]).toBe('/api/users/me');
	expect(fetchImpl.mock.calls[0][2]).toMatchObject({ method: 'PUT' });
	expect(JSON.parse(String(fetchImpl.mock.calls[0][2].body))).toEqual({ locale: 'en' });
});

test('an account that refuses the change still leaves the browser switched', async () => {
	// The cookie is what this request is for. Losing the account write costs
	// the other devices, not this page.
	fetchImpl.mockResolvedValue({ ok: false, status: 500, json: async () => ({}) });
	const e = event('en', { user_id: 'u-1' });

	const response = await POST(e as never);

	expect(response.status).toBe(204);
	expect(e.cookies.set).toHaveBeenCalled();
});

test('a language the product cannot speak is refused', async () => {
	// Not merely one it has never heard of — `id` is a locale this product
	// intends to speak and has no copy for yet. Storing it would leave somebody
	// switched to a language that renders in English.
	for (const refused of ['fr', 'id']) {
		const e = event(refused);

		expect((await POST(e as never)).status).toBe(400);
		expect(e.cookies.set).not.toHaveBeenCalled();
	}
});

test('nonsense in the body is refused rather than stored', async () => {
	const e = event({ nope: true });

	expect((await POST(e as never)).status).toBe(400);
	expect(e.cookies.set).not.toHaveBeenCalled();
});
