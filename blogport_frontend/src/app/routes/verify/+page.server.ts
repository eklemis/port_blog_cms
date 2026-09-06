import { redirect } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';

/**
 * The hold screen needs one thing the page cannot invent: the address the
 * verification link went to. It comes from the session.
 *
 * There is deliberately no "already verified, go to the console" branch here.
 * `GET /api/users/me` is what `hooks.server.ts` rebuilds the session from on
 * every request, and its `UserProfileResponse` carries no `is_verified` — only
 * the login response does, once. So a page load cannot tell whether this
 * account has since verified. `backend_actix/docs/AUTHENTICATION.md` says the
 * field is returned; the handler does not have it. See the PR.
 */
export const load: PageServerLoad = async ({ locals }) => {
	// Without a session there is no address, and the screen's first sentence is
	// the address — there is nothing honest left to render.
	if (!locals.user) redirect(303, '/auth/login');

	return { email: locals.user.email };
};
