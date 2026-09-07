import { redirect } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';
import { CONSOLE_ROUTE, SIGN_IN_ROUTE } from '$lib/shared/config/routes';

/**
 * The hold screen needs one thing the page cannot invent: the address the
 * verification link went to. It comes from the session.
 *
 * It also decides who should not be looking at it at all. `GET /api/users/me`
 * reads `is_verified` from the row rather than from the token claim, so the
 * answer is fresh — a token minted before someone opened the emailed link still
 * says `false`, and the endpoint says `true` anyway. Because hooks.server.ts
 * rebuilds the session from it on every request, this screen lets go of them on
 * their next navigation rather than stranding them until they sign in again.
 */
export const load: PageServerLoad = async ({ locals }) => {
	// Without a session there is no address, and the screen's first sentence is
	// the address — there is nothing honest left to render.
	if (!locals.user) redirect(303, SIGN_IN_ROUTE);

	// The screen exists to be left.
	if (locals.user.is_verified) redirect(303, CONSOLE_ROUTE);

	return { email: locals.user.email };
};
