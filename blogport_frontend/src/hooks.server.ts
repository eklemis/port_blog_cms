import type { Handle } from '@sveltejs/kit';
import { authenticatedFetch } from '$lib/shared/api/backend.server';
import { clearAuthCookies } from '$lib/shared/auth/cookies.server';
import { LOCALE_COOKIE, localeForRoute } from '$lib/shared/i18n';

export const handle: Handle = async ({ event, resolve }) => {
	try {
		const res = await authenticatedFetch(event, '/api/users/me');

		if (res.ok) {
			const json = await res.json();
			event.locals.user = json.data;
		} else {
			event.locals.user = null;
		}
	} catch {
		event.locals.user = null;
		clearAuthCookies(event.cookies);
	}

	/**
	 * Career Studio §02, as ruled on 20 September. Resolved here rather than in
	 * the browser because every route is server-rendered: a page that arrives in
	 * English and switches after paint is the flash the theme script exists to
	 * avoid, with the whole interface instead of the background.
	 *
	 * Three states, as the theme has: the cookie when somebody has chosen, and
	 * `Accept-Language` when nobody has — everywhere except the public shell,
	 * which is not localised at all. `localeForRoute` holds that rule and says
	 * why.
	 */
	event.locals.locale = localeForRoute({
		routeId: event.route.id,
		// An account outranks the browser it is being read in: somebody who set
		// their language on one machine should not have to set it again on the
		// next. The cookie is what the auth shell writes, since it has no session
		// to write to.
		stored: event.locals.user?.locale ?? event.cookies.get(LOCALE_COOKIE),
		header: event.request.headers.get('accept-language')
	});

	return resolve(event, {
		transformPageChunk: ({ html }) => html.replace('%lang%', event.locals.locale)
	});
};
