import { expect, test } from 'vitest';
import { isRedirect } from '@sveltejs/kit';
import { load } from './+layout.server';

/**
 * The console's gate, on the layout so every screen under it inherits one
 * answer rather than eight.
 *
 * Login is not the gate; verification is. `is_verified` is read from the
 * session, which hooks.server.ts rebuilds from `GET /api/users/me` on every
 * request — from the row rather than the token claim, so someone who verifies
 * in another tab is let in on their next navigation.
 */

const VERIFIED = {
	user_id: '123e4567-e89b-12d3-a456-426614174000',
	email: 'jane@example.com',
	username: 'janedoe',
	full_name: 'Jane Doe',
	bio: null,
	locale: 'en',
	is_verified: true
};

async function redirectFrom(event: unknown) {
	try {
		await load(event as never);
	} catch (thrown) {
		return thrown as { status: number; location: string };
	}
	throw new Error('expected a redirect, and the load returned instead');
}

function event(user: typeof VERIFIED | null, path = '/studio') {
	return { locals: { user }, url: new URL(`http://localhost${path}`) };
}

test('a verified author is let in, with their name for the greeting', async () => {
	const data = await load(event(VERIFIED) as never);

	expect(data).toMatchObject({ user: { full_name: 'Jane Doe' } });
});

test('an unverified account never reaches an authoring surface', async () => {
	// The rule the whole gate exists for: every authoring route 403s for them,
	// so a console rendered here would fail on every button.
	const thrown = await redirectFrom(event({ ...VERIFIED, is_verified: false }));

	expect(isRedirect(thrown)).toBe(true);
	expect(thrown.location).toBe('/verify');
});

test('a signed-out visitor goes to sign in', async () => {
	const thrown = await redirectFrom(event(null));

	expect(isRedirect(thrown)).toBe(true);
	expect(thrown.location).toMatch(/^\/auth\/login/);
});

test('and the destination they wanted is preserved', async () => {
	// J2: redirect to login with the intended path preserved, and return them
	// there afterwards. The sign-in gate already validates what it is handed.
	const thrown = await redirectFrom(event(null, '/studio/posts/new'));

	expect(thrown.location).toBe('/auth/login?next=%2Fstudio%2Fposts%2Fnew');
});

test('the gate applies to every screen under it, not just the overview', async () => {
	const thrown = await redirectFrom(event({ ...VERIFIED, is_verified: false }, '/studio/media'));

	expect(thrown.location).toBe('/verify');
});
