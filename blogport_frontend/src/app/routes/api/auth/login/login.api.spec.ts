import { beforeEach, expect, test, vi } from 'vitest';

const backendPOST = vi.fn();
const setAuthCookies = vi.fn();

vi.mock('$lib/shared/api/client', () => ({
	POST: (...args: unknown[]) => backendPOST(...args)
}));

vi.mock('$lib/shared/auth/cookies.server', async (importOriginal) => ({
	...(await importOriginal<Record<string, unknown>>()),
	setAuthCookies: (...args: unknown[]) => setAuthCookies(...args)
}));

const { POST } = await import('./+server');

/**
 * The proxy is the only thing that ever sees a token. It calls the backend,
 * puts both tokens in httpOnly cookies, and answers the browser with the user
 * and — when something failed — the error CODE, so the client can reach for the
 * Console Blueprint's sentence rather than reprinting the backend's prose.
 */

const TOKENS = {
	access_token: 'access.jwt.value',
	refresh_token: 'refresh.jwt.value'
};

const USER = {
	email: 'jane@example.com',
	id: '123e4567-e89b-12d3-a456-426614174000',
	is_verified: true,
	username: 'janedoe'
};

function event(body: unknown = { email: 'jane@example.com', password: 'a-real-password' }) {
	return {
		request: new Request('http://localhost/api/auth/login', {
			method: 'POST',
			body: JSON.stringify(body)
		}),
		cookies: {} as never
	};
}

function backendSucceeds(user = USER) {
	backendPOST.mockResolvedValue({
		data: { data: { ...TOKENS, user }, success: true },
		error: undefined,
		response: new Response(null, { status: 200 })
	});
}

function backendFails(status: number, code: string, message: string, headers = {}) {
	backendPOST.mockResolvedValue({
		data: undefined,
		error: { error: { code, message }, success: false },
		response: new Response(null, { status, headers })
	});
}

beforeEach(() => {
	backendPOST.mockReset();
	setAuthCookies.mockReset();
});

// ── success ────────────────────────────────────────────────────────────────

test('puts both tokens in cookies and returns only the user', async () => {
	backendSucceeds();

	const response = await POST(event() as never);

	expect(setAuthCookies).toHaveBeenCalledWith(expect.anything(), TOKENS);
	expect(await response.json()).toEqual({ user: USER });
});

test('no token reaches the browser body', async () => {
	backendSucceeds();

	const response = await POST(event() as never);

	// The whole point of the proxy. If a token appears here it is readable by
	// script, and httpOnly has bought nothing.
	expect(await response.text()).not.toMatch(/jwt\.value/);
});

test('login succeeds for an unverified account, tokens and all', async () => {
	// Login is not the gate — refusing here would be the wrong fix for the wrong
	// problem. The tokens are real and work; every authoring route is what says
	// no, with 403 EMAIL_NOT_VERIFIED. Console Blueprint §02.
	backendSucceeds({ ...USER, is_verified: false });

	const response = await POST(event() as never);

	expect(setAuthCookies).toHaveBeenCalledWith(expect.anything(), TOKENS);
	expect(response.status).toBe(200);
	expect(await response.json()).toEqual({ user: { ...USER, is_verified: false } });
});

test('the verification flag survives the trip', async () => {
	// It is what the client routes on; dropping it from the payload would send
	// an unverified account to the console.
	backendSucceeds({ ...USER, is_verified: false });

	const body = (await (await POST(event() as never)).json()) as { user: { is_verified: boolean } };

	expect(body.user.is_verified).toBe(false);
});

// ── failure ────────────────────────────────────────────────────────────────

test('forwards the error code, not just the message', async () => {
	backendFails(401, 'INVALID_CREDENTIALS', 'Invalid email or password');

	const response = await POST(event() as never);

	expect(response.status).toBe(401);
	expect(await response.json()).toEqual({
		error: { code: 'INVALID_CREDENTIALS', message: 'Invalid email or password' }
	});
});

test('a failed sign-in sets no cookies', async () => {
	backendFails(401, 'INVALID_CREDENTIALS', 'Invalid email or password');

	await POST(event() as never);

	expect(setAuthCookies).not.toHaveBeenCalled();
});

test('passes Retry-After through so the countdown is the real one', async () => {
	backendFails(429, 'RATE_LIMITED', 'Too many requests', { 'retry-after': '300' });

	const response = await POST(event() as never);

	expect(response.status).toBe(429);
	expect(response.headers.get('retry-after')).toBe('300');
});

test('a closed account keeps its own code rather than becoming a 401', async () => {
	backendFails(403, 'USER_DELETED', 'Account has been deleted');

	const response = await POST(event() as never);

	expect(response.status).toBe(403);
	expect(await response.json()).toMatchObject({ error: { code: 'USER_DELETED' } });
});

test('an unreachable backend answers with a code instead of an unhandled throw', async () => {
	backendPOST.mockRejectedValue(new TypeError('fetch failed'));

	const response = await POST(event() as never);

	expect(response.status).toBe(502);
	expect(await response.json()).toMatchObject({ error: { code: 'INTERNAL_ERROR' } });
	expect(setAuthCookies).not.toHaveBeenCalled();
});

test('an error body the backend did not shape still yields a code', async () => {
	backendPOST.mockResolvedValue({
		data: undefined,
		error: 'plain text, not our envelope',
		response: new Response(null, { status: 500 })
	});

	const response = await POST(event() as never);

	expect(response.status).toBe(500);
	expect(await response.json()).toMatchObject({ error: { code: 'INTERNAL_ERROR' } });
});
