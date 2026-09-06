import { expect, test, vi } from 'vitest';
import { UNEXPECTED } from '$lib/shared/lib/api-failure';
import { ACCOUNT_CLOSED, SIGN_IN_FAILED, signIn } from './sign-in';

/**
 * The browser talks to `/api/auth/login`, the SvelteKit proxy — never to the
 * backend, and never holding a JWT. What comes back is a user or a code; this
 * module turns the code into the Console Blueprint §07 sentence for it.
 */

const USER = {
	email: 'jane@example.com',
	id: '123e4567-e89b-12d3-a456-426614174000',
	is_verified: true,
	username: 'janedoe'
};

function respond(status: number, body: unknown, headers: Record<string, string> = {}) {
	return vi.fn<typeof fetch>(async () => new Response(JSON.stringify(body), { status, headers }));
}

// ── the request ────────────────────────────────────────────────────────────

test('posts the credentials to the proxy route, as JSON', async () => {
	const fetchFn = respond(200, { user: USER });

	await signIn({ email: 'jane@example.com', password: 'a-real-password' }, fetchFn);

	expect(fetchFn).toHaveBeenCalledOnce();
	const [url, init] = fetchFn.mock.calls[0];
	expect(url).toBe('/api/auth/login');
	expect(init?.method).toBe('POST');
	expect(JSON.parse(init?.body as string)).toEqual({
		email: 'jane@example.com',
		password: 'a-real-password'
	});
});

test('trims the address but never the password', async () => {
	const fetchFn = respond(200, { user: USER });

	await signIn({ email: '  jane@example.com ', password: '  spaces are secret  ' }, fetchFn);

	const [, init] = fetchFn.mock.calls[0];
	expect(JSON.parse(init?.body as string)).toEqual({
		email: 'jane@example.com',
		password: '  spaces are secret  '
	});
});

// ── success ────────────────────────────────────────────────────────────────

test('returns the user, verification flag and all', async () => {
	const fetchFn = respond(200, { user: { ...USER, is_verified: false } });

	const result = await signIn({ email: 'jane@example.com', password: 'x' }, fetchFn);

	expect(result).toEqual({ ok: true, user: { ...USER, is_verified: false } });
});

test('carries no token back to the browser', async () => {
	// The proxy sets httpOnly cookies and returns the user only. If a token ever
	// appears in this payload, the cookie pattern has been undone.
	const fetchFn = respond(200, { user: USER });

	const result = await signIn({ email: 'jane@example.com', password: 'x' }, fetchFn);

	expect(JSON.stringify(result)).not.toMatch(/token/i);
});

// ── the deliberate ambiguity ───────────────────────────────────────────────

test('a failed sign-in says one thing, in the product’s words', async () => {
	const fetchFn = respond(401, {
		error: { code: 'INVALID_CREDENTIALS', message: 'Invalid email or password' }
	});

	const result = await signIn({ email: 'nobody@example.com', password: 'x' }, fetchFn);

	// Not the backend's prose — the sentence the Console Blueprint chose.
	expect(result).toEqual({ ok: false, message: SIGN_IN_FAILED, retryAfterSeconds: null });
	expect(SIGN_IN_FAILED).toBe("That email and password don't match.");
});

test('an unknown address and a wrong password are indistinguishable', async () => {
	// The whole point. The backend answers INVALID_CREDENTIALS for both so the
	// endpoint cannot be used to discover which addresses have accounts, and
	// nothing on this side may narrow it back down.
	const unknownAddress = await signIn(
		{ email: 'nobody@example.com', password: 'a-real-password' },
		respond(401, { error: { code: 'INVALID_CREDENTIALS', message: 'Invalid email or password' } })
	);
	const wrongPassword = await signIn(
		{ email: 'jane@example.com', password: 'wrong' },
		respond(401, { error: { code: 'INVALID_CREDENTIALS', message: 'No such password' } })
	);

	expect(unknownAddress).toEqual(wrongPassword);
});

// ── rate limiting ──────────────────────────────────────────────────────────

test('a rate limit reports how long, from Retry-After', async () => {
	const fetchFn = respond(
		429,
		{ error: { code: 'RATE_LIMITED', message: 'Too many requests' } },
		{ 'retry-after': '300' }
	);

	const result = await signIn({ email: 'jane@example.com', password: 'x' }, fetchFn);

	expect(result).toEqual({
		ok: false,
		message: 'Too many attempts. Try again in 5 minutes.',
		retryAfterSeconds: 300
	});
});

test('a rate limit with no Retry-After still says something useful', async () => {
	const fetchFn = respond(429, { error: { code: 'RATE_LIMITED', message: 'Too many requests' } });

	const result = await signIn({ email: 'jane@example.com', password: 'x' }, fetchFn);

	expect(result.ok).toBe(false);
	expect(result).toMatchObject({ message: 'Too many attempts. Try again shortly.' });
});

// ── the other branches ─────────────────────────────────────────────────────

test('a closed account is not a failed password', async () => {
	const fetchFn = respond(403, {
		error: { code: 'USER_DELETED', message: 'Account has been deleted' }
	});

	const result = await signIn({ email: 'jane@example.com', password: 'x' }, fetchFn);

	expect(result).toEqual({ ok: false, message: ACCOUNT_CLOSED, retryAfterSeconds: null });
});

test('a server fault is ours, and says so', async () => {
	const fetchFn = respond(500, {
		error: { code: 'INTERNAL_ERROR', message: 'An unexpected error occurred' }
	});

	const result = await signIn({ email: 'jane@example.com', password: 'x' }, fetchFn);

	expect(result).toEqual({ ok: false, message: UNEXPECTED, retryAfterSeconds: null });
	expect(UNEXPECTED).toBe('Something went wrong on our side.');
});

test('a dead network is not a raw exception in the user’s face', async () => {
	const fetchFn = vi.fn<typeof fetch>(async () => {
		throw new TypeError('Failed to fetch');
	});

	const result = await signIn({ email: 'jane@example.com', password: 'x' }, fetchFn);

	expect(result).toEqual({ ok: false, message: UNEXPECTED, retryAfterSeconds: null });
});

test('a response that is not JSON at all falls back rather than throwing', async () => {
	const fetchFn = vi.fn<typeof fetch>(
		async () => new Response('<html>502 Bad Gateway</html>', { status: 502 })
	);

	const result = await signIn({ email: 'jane@example.com', password: 'x' }, fetchFn);

	expect(result).toEqual({ ok: false, message: UNEXPECTED, retryAfterSeconds: null });
});

test('an unrecognised code never reaches the screen as a code', async () => {
	// "Never render a raw error code" — Console Blueprint §06.
	const fetchFn = respond(400, {
		error: { code: 'SOME_CODE_ADDED_LATER', message: 'whatever the server said' }
	});

	const result = await signIn({ email: 'jane@example.com', password: 'x' }, fetchFn);

	expect(result).toEqual({ ok: false, message: UNEXPECTED, retryAfterSeconds: null });
});
