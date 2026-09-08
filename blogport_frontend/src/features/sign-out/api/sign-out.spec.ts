import { expect, test, vi } from 'vitest';
import { LOGOUT_ROUTE, signOut } from './sign-out';

/**
 * Signing out. The refresh token is blacklisted server-side and the cookies are
 * cleared by the proxy; what matters on this side is that the person ends up
 * out, whatever the network did.
 */

test('asks the proxy to end the session', async () => {
	const fetchFn = vi.fn<typeof fetch>(async () => new Response(JSON.stringify({ ok: true })));

	await signOut(fetchFn);

	expect(fetchFn).toHaveBeenCalledOnce();
	const [url, init] = fetchFn.mock.calls[0];
	expect(url).toBe(LOGOUT_ROUTE);
	expect(init?.method).toBe('POST');
});

test('a failed logout never traps someone in a session they asked to end', async () => {
	// J2 says clear local state regardless of the response, and the backend
	// confirms it: a missing or unverifiable token is logged and still answers
	// 200. So there is no failure mode worth reporting, and refusing to move on
	// would strand the one person who most wants to leave.
	const fetchFn = vi.fn<typeof fetch>(async () => {
		throw new TypeError('Failed to fetch');
	});

	await expect(signOut(fetchFn)).resolves.toBeUndefined();
});

test('a rejection from the server is not raised either', async () => {
	const fetchFn = vi.fn<typeof fetch>(async () => new Response(null, { status: 500 }));

	await expect(signOut(fetchFn)).resolves.toBeUndefined();
});
