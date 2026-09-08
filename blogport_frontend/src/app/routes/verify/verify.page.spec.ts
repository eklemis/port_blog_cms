import { expect, test } from 'vitest';
import { isRedirect } from '@sveltejs/kit';
import { CONSOLE_ROUTE, SIGN_IN_ROUTE } from '$lib/shared/config/routes';
import { load } from './+page.server';

/**
 * What the hold screen needs before it can render, and who is turned away.
 *
 * The verified branch is the interesting one. `GET /api/users/me` reads
 * `is_verified` from the row rather than from the token claim, so the answer is
 * fresh: someone who opens the emailed link in another tab is carrying a token
 * that still says `false`, and this endpoint says `true` anyway. Since
 * hooks.server.ts rebuilds the session from it per request, the hold screen
 * lets go of them the moment they verify, without a sign-out and back in.
 */

const SESSION = {
	user_id: '123e4567-e89b-12d3-a456-426614174000',
	email: 'jane@example.com',
	username: 'janedoe',
	full_name: 'Jane Doe',
	bio: null,
	locale: 'en',
	is_verified: false
};

/** A cookie jar holding whatever the register proxy left behind. */
function jar(pending?: string) {
	return { get: (name: string) => (name === 'pending_verification_email' ? pending : undefined) };
}

async function redirectFrom(event: unknown) {
	try {
		await load(event as never);
	} catch (thrown) {
		return thrown as { status: number; location: string };
	}
	throw new Error('expected a redirect, and the load returned instead');
}

test('hands the screen the address to name', async () => {
	const data = await load({ locals: { user: SESSION }, cookies: jar() } as never);

	expect(data).toEqual({ email: 'jane@example.com' });
});

test('takes the address from registration when there is no session yet', async () => {
	// J1 routes here straight after registering, which returns a user and no
	// token. Without this the screen could not name the address it just used.
	const data = await load({
		locals: { user: null },
		cookies: jar('new@example.com')
	} as never);

	expect(data).toEqual({ email: 'new@example.com' });
});

test('a real session outranks whatever registration left behind', async () => {
	const data = await load({
		locals: { user: SESSION },
		cookies: jar('someone-else@example.com')
	} as never);

	expect(data).toEqual({ email: 'jane@example.com' });
});

test('an account that has since verified is let go, not held', async () => {
	// The screen exists to be left. Holding someone who has already opened the
	// link would be the dead end this whole gate is designed to avoid.
	const thrown = await redirectFrom({
		locals: { user: { ...SESSION, is_verified: true } },
		cookies: jar()
	});

	expect(isRedirect(thrown)).toBe(true);
	expect(thrown.location).toBe(CONSOLE_ROUTE);
	expect(CONSOLE_ROUTE).toBe('/studio');
});

test('sends a visitor with no address at all to sign in', async () => {
	// The screen's first sentence is the address the link went to. With neither
	// a session nor a registration behind them there is no address, and nothing
	// honest left to render.
	const thrown = await redirectFrom({ locals: { user: null }, cookies: jar() });

	expect(isRedirect(thrown)).toBe(true);
	expect(thrown.location).toBe(SIGN_IN_ROUTE);
	expect(SIGN_IN_ROUTE).toBe('/auth/login');
});
