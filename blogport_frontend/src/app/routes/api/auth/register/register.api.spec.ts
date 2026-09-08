import { beforeEach, expect, test, vi } from 'vitest';

const backendPOST = vi.fn();
const setPendingVerificationEmail = vi.fn();

/** Set to make the client throw — see resend.api.spec.ts for why not a mock rejection. */
let unreachable: Error | null = null;

vi.mock('$lib/shared/api/client', () => ({
	POST: async (...args: unknown[]) => {
		if (unreachable) throw unreachable;
		return backendPOST(...args);
	}
}));

vi.mock('$lib/shared/auth/cookies.server', async (importOriginal) => ({
	...(await importOriginal<Record<string, unknown>>()),
	setPendingVerificationEmail: (...args: unknown[]) => setPendingVerificationEmail(...args)
}));

const { POST } = await import('./+server');

/**
 * `POST /api/auth/register`.
 *
 * Registration returns a user and no token, so nothing here establishes a
 * session. What it does leave behind is the address the link went to, because
 * the hold screen is the next thing the person sees and it has no other way to
 * know it.
 */

/**
 * Long enough to clear the only rule, and named rather than written inline so
 * it reads as what it is: a fixture, not anybody's password.
 */
const LONG_ENOUGH = 'fixture-12-chars-min';

const NEW_USER = {
	email: 'jane@example.com',
	full_name: 'Jane Doe',
	id: '123e4567-e89b-12d3-a456-426614174000',
	username: 'janedoe'
};

function event(body: unknown = { ...NEW_USER, password: LONG_ENOUGH }) {
	return {
		request: new Request('http://localhost/api/auth/register', {
			method: 'POST',
			body: JSON.stringify(body)
		}),
		cookies: {} as never
	};
}

function backendCreates() {
	backendPOST.mockResolvedValue({
		data: { data: { message: 'User created', user: NEW_USER }, success: true },
		error: undefined,
		response: new Response(null, { status: 201 })
	});
}

function backendRefuses(status: number, code: string, message = 'whatever', headers = {}) {
	backendPOST.mockResolvedValue({
		data: undefined,
		error: { error: { code, message }, success: false },
		response: new Response(null, { status, headers })
	});
}

beforeEach(() => {
	backendPOST.mockReset();
	setPendingVerificationEmail.mockReset();
	unreachable = null;
});

test('creates the account and answers with the user', async () => {
	backendCreates();

	const response = await POST(event() as never);

	expect(response.status).toBe(201);
	expect(await response.json()).toMatchObject({ user: NEW_USER });
});

test('remembers the address the link went to', async () => {
	// The hold screen is next and has no session to read it from.
	backendCreates();

	await POST(event() as never);

	expect(setPendingVerificationEmail).toHaveBeenCalledWith(expect.anything(), 'jane@example.com');
});

test('no password is echoed back to the browser', async () => {
	backendCreates();

	const response = await POST(event() as never);

	expect(await response.text()).not.toMatch(LONG_ENOUGH);
});

test('forwards the collision code so the screen can offer the way out', async () => {
	// 409 is the branch that needs "Sign in instead" and "Reset your password";
	// the message alone cannot be branched on.
	backendRefuses(409, 'USER_ALREADY_EXISTS', 'User already exists');

	const response = await POST(event() as never);

	expect(response.status).toBe(409);
	expect(await response.json()).toEqual({
		error: { code: 'USER_ALREADY_EXISTS', message: 'User already exists' }
	});
});

test('a refused registration remembers nothing', async () => {
	backendRefuses(409, 'USER_ALREADY_EXISTS');

	await POST(event() as never);

	expect(setPendingVerificationEmail).not.toHaveBeenCalled();
});

test('forwards a field code with the server’s own reason', async () => {
	// J1: the server's message names the failed rule. Ours mirrors the rule, so
	// if the server still refuses, its reason is the one worth reading.
	backendRefuses(400, 'INVALID_PASSWORD', 'Password must be at least 12 characters');

	const response = await POST(event() as never);

	expect(await response.json()).toMatchObject({
		error: { code: 'INVALID_PASSWORD', message: 'Password must be at least 12 characters' }
	});
});

test('passes Retry-After through for the sign-up countdown', async () => {
	backendRefuses(429, 'RATE_LIMITED', 'Too many requests', { 'retry-after': '2520' });

	const response = await POST(event() as never);

	expect(response.status).toBe(429);
	expect(response.headers.get('retry-after')).toBe('2520');
});

test('an unreachable backend answers with a code, not an unhandled throw', async () => {
	unreachable = new TypeError('fetch failed');

	const response = await POST(event() as never);

	expect(response.status).toBe(502);
	expect(await response.json()).toMatchObject({ error: { code: 'INTERNAL_ERROR' } });
});
