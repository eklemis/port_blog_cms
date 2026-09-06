import { expect, test } from 'vitest';
import { isRedirect } from '@sveltejs/kit';
import { load } from './+page.server';

/**
 * What the hold screen needs before it can render, and who is turned away.
 *
 * Note what is NOT here: a redirect for an account that has already verified.
 * `GET /api/users/me` does not return `is_verified` — the session is rebuilt
 * from that endpoint on every request, so a page load cannot tell. See the PR;
 * `backend_actix/docs/AUTHENTICATION.md` claims the field is returned and the
 * handler does not have it.
 */

const SESSION = {
	user_id: '123e4567-e89b-12d3-a456-426614174000',
	email: 'jane@example.com',
	username: 'janedoe',
	full_name: 'Jane Doe',
	bio: null,
	locale: 'en'
};

async function redirectFrom(event: unknown) {
	try {
		await load(event as never);
	} catch (thrown) {
		return thrown as { status: number; location: string };
	}
	throw new Error('expected a redirect, and the load returned instead');
}

test('hands the screen the address to name', async () => {
	const data = await load({ locals: { user: SESSION } } as never);

	expect(data).toEqual({ email: 'jane@example.com' });
});

test('sends a signed-out visitor to sign in', async () => {
	// The screen's first sentence is the address the link went to; with no
	// session there is no address, and nothing honest left to render.
	const thrown = await redirectFrom({ locals: { user: null } });

	expect(isRedirect(thrown)).toBe(true);
	expect(thrown.location).toBe('/auth/login');
});
