import { redirect } from '@sveltejs/kit';
import type { LayoutServerLoad } from './$types';
import { HOLD_ROUTE, SIGN_IN_ROUTE } from '$lib/shared/config/routes';

/**
 * The console's gate.
 *
 * On the layout, so every screen beneath it inherits one answer rather than
 * repeating the check eight times and getting it wrong once.
 *
 * Login is not the gate; verification is. `POST /api/auth/login` succeeds for
 * an unverified account and hands back working tokens, and every authoring
 * route then refuses them with 403 EMAIL_NOT_VERIFIED — so a console rendered
 * for one would fail on every button. Console Blueprint §02.
 */
export const load: LayoutServerLoad = async ({ locals, url }) => {
	if (!locals.user) {
		// J2: preserve where they were going, and return them there after.
		const next = `${url.pathname}${url.search}`;
		redirect(303, `${SIGN_IN_ROUTE}?next=${encodeURIComponent(next)}`);
	}

	if (!locals.user.is_verified) redirect(303, HOLD_ROUTE);

	return { user: locals.user };
};
