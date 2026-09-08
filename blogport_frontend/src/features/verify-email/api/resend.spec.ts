import { expect, test, vi } from 'vitest';
import { UNEXPECTED } from '$lib/shared/lib/api-failure';
import { RESEND_ROUTE, SESSION_EXPIRED, resendVerification } from './resend';

/**
 * Resending the verification link. The address is optional: the hold screen has
 * a session for the proxy to read it from, and the expired-link screen has none
 * and asks for it.
 */

function respond(status: number, body: unknown, headers: Record<string, string> = {}) {
	return vi.fn<typeof fetch>(async () => new Response(JSON.stringify(body), { status, headers }));
}

test('sends no address when it has none — the proxy uses the session', async () => {
	const fetchFn = respond(202, {});

	await resendVerification(undefined, fetchFn);

	const [url, init] = fetchFn.mock.calls[0];
	expect(url).toBe(RESEND_ROUTE);
	expect(init?.method).toBe('POST');
	expect(JSON.parse(init?.body as string)).toEqual({});
});

test('sends the address it was given, trimmed', async () => {
	const fetchFn = respond(202, {});

	await resendVerification('  typed@example.com  ', fetchFn);

	const [, init] = fetchFn.mock.calls[0];
	expect(JSON.parse(init?.body as string)).toEqual({ email: 'typed@example.com' });
});

test('accepts without repeating the backend’s sentence', async () => {
	// An API message is written for a developer; what the person reads is the
	// copy table's, and which sentence that is depends on the screen.
	const fetchFn = respond(202, { message: 'Email verified successfully' });

	expect(await resendVerification(undefined, fetchFn)).toEqual({ ok: true });
});

test('a rate limit counts down against the real Retry-After', async () => {
	// Five per hour, matching password reset — each call mints a token and sends
	// mail, so this is a limit someone waiting on an email will actually meet.
	const fetchFn = respond(429, { error: { code: 'RATE_LIMITED' } }, { 'retry-after': '3600' });

	expect(await resendVerification(undefined, fetchFn)).toEqual({
		ok: false,
		message: 'Too many attempts. Try again in 60 minutes.',
		retryAfterSeconds: 3600,
		signedOut: false
	});
});

test('a lost session is reported as one, not as a server fault', async () => {
	// The hold screen is a waiting screen — someone can sit on it for a long
	// time. "Something went wrong on our side" would be a lie and a dead end.
	const fetchFn = respond(401, { error: { code: 'MISSING_AUTH_HEADER' } });

	expect(await resendVerification(undefined, fetchFn)).toEqual({
		ok: false,
		message: SESSION_EXPIRED,
		retryAfterSeconds: null,
		signedOut: true
	});
	expect(SESSION_EXPIRED).toBe('Your session expired. Sign in to pick up where you left off.');
});

test('a server fault is ours, and says so', async () => {
	const fetchFn = respond(500, { error: { code: 'INTERNAL_ERROR' } });

	expect(await resendVerification(undefined, fetchFn)).toEqual({
		ok: false,
		message: UNEXPECTED,
		retryAfterSeconds: null,
		signedOut: false
	});
});

test('a dead network is not a raw exception in the user’s face', async () => {
	const fetchFn = vi.fn<typeof fetch>(async () => {
		throw new TypeError('Failed to fetch');
	});

	expect(await resendVerification(undefined, fetchFn)).toMatchObject({
		ok: false,
		message: UNEXPECTED
	});
});

test('a response that is not JSON falls back rather than throwing', async () => {
	const fetchFn = vi.fn<typeof fetch>(
		async () => new Response('<html>502</html>', { status: 502 })
	);

	expect(await resendVerification(undefined, fetchFn)).toMatchObject({
		ok: false,
		message: UNEXPECTED
	});
});
