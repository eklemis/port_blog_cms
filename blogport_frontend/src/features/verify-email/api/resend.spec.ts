import { expect, test, vi } from 'vitest';
import { UNEXPECTED } from '$lib/shared/lib/api-failure';
import { RESEND_ROUTE, SESSION_EXPIRED, resendVerification } from './resend';

/**
 * Resending the verification link. One action, no arguments — the address lives
 * in the session and the proxy reads it there, so there is nothing for the
 * browser to pass and nothing for it to get wrong.
 */

const ACCEPTED = 'If that address needs verifying, a new link is on its way.';

function respond(status: number, body: unknown, headers: Record<string, string> = {}) {
	return vi.fn<typeof fetch>(async () => new Response(JSON.stringify(body), { status, headers }));
}

test('posts to the proxy route with no body of its own', async () => {
	const fetchFn = respond(202, { message: ACCEPTED });

	await resendVerification(fetchFn);

	const [url, init] = fetchFn.mock.calls[0];
	expect(url).toBe(RESEND_ROUTE);
	expect(init?.method).toBe('POST');
	// No address on the wire: passing one would invite the caller to choose it.
	expect(init?.body).toBeUndefined();
});

test('reports what the endpoint said when it accepts', async () => {
	const fetchFn = respond(202, { message: ACCEPTED });

	expect(await resendVerification(fetchFn)).toEqual({ ok: true, message: ACCEPTED });
});

test('still says something when the response carries no message', async () => {
	const fetchFn = respond(202, {});

	const result = await resendVerification(fetchFn);

	expect(result.ok).toBe(true);
	expect(result.message.length).toBeGreaterThan(0);
});

test('a rate limit counts down against the real Retry-After', async () => {
	// Five per hour, matching password reset — each call mints a token and sends
	// mail, so this is a limit someone waiting on an email will actually meet.
	const fetchFn = respond(429, { error: { code: 'RATE_LIMITED' } }, { 'retry-after': '3600' });

	expect(await resendVerification(fetchFn)).toEqual({
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

	expect(await resendVerification(fetchFn)).toEqual({
		ok: false,
		message: SESSION_EXPIRED,
		retryAfterSeconds: null,
		signedOut: true
	});
	expect(SESSION_EXPIRED).toBe('Your session expired. Sign in to pick up where you left off.');
});

test('a server fault is ours, and says so', async () => {
	const fetchFn = respond(500, { error: { code: 'INTERNAL_ERROR' } });

	expect(await resendVerification(fetchFn)).toEqual({
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

	expect(await resendVerification(fetchFn)).toMatchObject({ ok: false, message: UNEXPECTED });
});

test('a response that is not JSON falls back rather than throwing', async () => {
	const fetchFn = vi.fn<typeof fetch>(
		async () => new Response('<html>502</html>', { status: 502 })
	);

	expect(await resendVerification(fetchFn)).toMatchObject({ ok: false, message: UNEXPECTED });
});
