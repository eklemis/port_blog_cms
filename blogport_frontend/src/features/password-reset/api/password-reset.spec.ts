import { expect, test, vi } from 'vitest';
import { UNEXPECTED } from '$lib/shared/lib/api-failure';
import {
	REQUEST_ROUTE,
	RESET_LINK_DEAD,
	requestReset,
	resetRequested,
	setPassword
} from './password-reset';

/**
 * Recovering a password. The whole point of J3 is that a stranger cannot learn
 * from it whether an address is registered, so the interesting assertions here
 * are about what is *not* reported.
 */

function respond(status: number, body: unknown, headers: Record<string, string> = {}) {
	return vi.fn<typeof fetch>(async () => new Response(JSON.stringify(body), { status, headers }));
}

// ── asking for a link ──────────────────────────────────────────────────────

test('posts the address to the proxy route', async () => {
	const fetchFn = respond(200, { ok: true });

	await requestReset('  Jane@Example.com ', fetchFn);

	const [url, init] = fetchFn.mock.calls[0];
	expect(url).toBe(REQUEST_ROUTE);
	expect(JSON.parse(init?.body as string)).toEqual({ email: 'Jane@Example.com' });
});

test('the confirmation says "if", and names the address without confirming it', async () => {
	// The address is echoed because the person just typed it — that reveals
	// nothing. What must never appear is whether it was found.
	const said = resetRequested('jane@example.com');

	expect(said).toBe(
		'If an account exists for jane@example.com, a reset link is on its way. It is good for one hour.'
	);
	expect(said).toMatch(/^If an account exists/);
});

test('an unknown address and a real one are indistinguishable', async () => {
	// The backend answers 200 either way; nothing here may narrow it.
	const unknown = await requestReset('nobody@example.com', respond(200, { ok: true }));
	const real = await requestReset('jane@example.com', respond(200, { ok: true }));

	expect(unknown).toEqual(real);
	expect(unknown).toEqual({ ok: true });
});

test('a rate limit is reported with a real countdown', async () => {
	const fetchFn = respond(429, { error: { code: 'RATE_LIMITED' } }, { 'retry-after': '3600' });

	expect(await requestReset('jane@example.com', fetchFn)).toEqual({
		ok: false,
		message: 'Too many attempts. Try again in 60 minutes.',
		retryAfterSeconds: 3600,
		expired: false
	});
});

test('a dead network is not a raw exception in the user’s face', async () => {
	const fetchFn = vi.fn<typeof fetch>(async () => {
		throw new TypeError('Failed to fetch');
	});

	expect(await requestReset('jane@example.com', fetchFn)).toMatchObject({
		ok: false,
		message: UNEXPECTED
	});
});

// ── using the link ─────────────────────────────────────────────────────────

test('sends the new password with the token in the path', async () => {
	const fetchFn = respond(200, { ok: true });

	await setPassword('the-emailed-token', 'a new long passphrase', fetchFn);

	const [url, init] = fetchFn.mock.calls[0];
	expect(url).toBe(`${REQUEST_ROUTE}/the-emailed-token`);
	// Never trimmed: the spaces someone typed are part of it.
	expect(JSON.parse(init?.body as string)).toEqual({ password: 'a new long passphrase' });
});

test('escapes the token rather than pasting it into a URL', async () => {
	const fetchFn = respond(200, { ok: true });

	await setPassword('a/token?with=chars', 'a new long passphrase', fetchFn);

	expect(fetchFn.mock.calls[0][0]).toBe(`${REQUEST_ROUTE}/a%2Ftoken%3Fwith%3Dchars`);
});

test('a stale link is reported as one, so the screen can offer a fresh one', async () => {
	const fetchFn = respond(401, { error: { code: 'INVALID_RESET_TOKEN' } });

	expect(await setPassword('t', 'a new long passphrase', fetchFn)).toEqual({
		ok: false,
		message: RESET_LINK_DEAD,
		retryAfterSeconds: null,
		expired: true
	});
	expect(RESET_LINK_DEAD).toBe('This reset link is no longer valid.');
});

test('a refused password keeps the token usable and says why', async () => {
	// J3: losing a valid token to one weak password is a needless restart, so
	// this is not the expired branch and the server's reason is what is shown.
	const fetchFn = respond(400, {
		error: { code: 'INVALID_PASSWORD', message: 'Password must be at least 12 characters' }
	});

	expect(await setPassword('t', 'short', fetchFn)).toEqual({
		ok: false,
		message: 'Password must be at least 12 characters',
		retryAfterSeconds: null,
		expired: false
	});
});

test('success carries no token, because the reset returns none', async () => {
	const fetchFn = respond(200, { ok: true });

	const result = await setPassword('t', 'a new long passphrase', fetchFn);

	expect(result).toEqual({ ok: true });
	expect(JSON.stringify(result)).not.toMatch(/token/i);
});
