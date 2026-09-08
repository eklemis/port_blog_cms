import { expect, test, vi } from 'vitest';
import { UNEXPECTED } from '$lib/shared/lib/api-failure';
import { ADDRESS_TAKEN, REGISTER_ROUTE, register } from './register';

/**
 * Creating the account. The browser talks to the SvelteKit proxy, which leaves
 * the address behind for the hold screen and hands back a user — never a token,
 * because registration does not sign anyone in.
 */

/**
 * Long enough to clear the only rule, and named rather than written inline so
 * it reads as what it is: a fixture, not anybody's password.
 */
const LONG_ENOUGH = 'fixture-12-chars-min';

const DETAILS = {
	username: 'janedoe',
	email: 'jane@example.com',
	full_name: 'Jane Doe',
	password: LONG_ENOUGH
};

function respond(status: number, body: unknown, headers: Record<string, string> = {}) {
	return vi.fn<typeof fetch>(async () => new Response(JSON.stringify(body), { status, headers }));
}

test('posts the details to the proxy route', async () => {
	const fetchFn = respond(201, { user: { username: 'janedoe' } });

	await register(DETAILS, fetchFn);

	const [url, init] = fetchFn.mock.calls[0];
	expect(url).toBe(REGISTER_ROUTE);
	expect(init?.method).toBe('POST');
	expect(JSON.parse(init?.body as string)).toEqual(DETAILS);
});

test('normalises the address and the username, but never the password', async () => {
	const fetchFn = respond(201, { user: {} });

	await register(
		{ ...DETAILS, username: '  JaneDoe ', email: '  Jane@Example.com ', password: '  spaced  ' },
		fetchFn
	);

	const [, init] = fetchFn.mock.calls[0];
	expect(JSON.parse(init?.body as string)).toMatchObject({
		username: 'janedoe',
		email: 'Jane@Example.com',
		password: '  spaced  '
	});
});

test('reports success without a token, because there is none', async () => {
	const fetchFn = respond(201, { user: { username: 'janedoe' } });

	const result = await register(DETAILS, fetchFn);

	expect(result).toEqual({ ok: true });
});

// ── the branch that matters ────────────────────────────────────────────────

test('an address already registered is a field-level collision', async () => {
	// Not a form-level failure: it belongs under the email field, with the two
	// ways out beside it.
	const fetchFn = respond(409, {
		error: { code: 'USER_ALREADY_EXISTS', message: 'User already exists' }
	});

	expect(await register(DETAILS, fetchFn)).toEqual({
		ok: false,
		field: 'email',
		message: ADDRESS_TAKEN,
		collision: true,
		retryAfterSeconds: null
	});
	expect(ADDRESS_TAKEN).toBe("There's already an account for that address.");
});

// ── field codes ────────────────────────────────────────────────────────────

test.each([
	['INVALID_USERNAME', 'username'],
	['INVALID_EMAIL', 'email'],
	['INVALID_FULL_NAME', 'full_name'],
	['INVALID_PASSWORD', 'password']
])('%s lands under the field it is about', async (code, field) => {
	// J1: the server's message names the failed rule. Ours mirrors the rule, so
	// reaching here means they disagreed and the server's reason is the useful
	// one to show.
	const fetchFn = respond(400, { error: { code, message: 'the server’s reason' } });

	expect(await register(DETAILS, fetchFn)).toMatchObject({
		ok: false,
		field,
		message: 'the server’s reason'
	});
});

// ── the rest ───────────────────────────────────────────────────────────────

test('a rate limit names the action that was limited', async () => {
	const fetchFn = respond(429, { error: { code: 'RATE_LIMITED' } }, { 'retry-after': '2520' });

	expect(await register(DETAILS, fetchFn)).toEqual({
		ok: false,
		field: null,
		message: 'Too many sign-up attempts. Try again in 42 minutes.',
		collision: false,
		retryAfterSeconds: 2520
	});
});

test('a server fault is ours, and belongs to no field', async () => {
	const fetchFn = respond(500, { error: { code: 'INTERNAL_ERROR' } });

	expect(await register(DETAILS, fetchFn)).toMatchObject({
		ok: false,
		field: null,
		message: UNEXPECTED
	});
});

test('a dead network is not a raw exception in the user’s face', async () => {
	const fetchFn = vi.fn<typeof fetch>(async () => {
		throw new TypeError('Failed to fetch');
	});

	expect(await register(DETAILS, fetchFn)).toMatchObject({ ok: false, message: UNEXPECTED });
});
