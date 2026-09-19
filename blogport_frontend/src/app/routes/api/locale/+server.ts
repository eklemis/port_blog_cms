import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { authenticatedFetch } from '$lib/shared/api/backend.server';
import { LOCALE_COOKIE, availableLocales, isLocale } from '$lib/shared/i18n';

/**
 * `POST /api/locale` — where the switcher writes.
 *
 * Career Studio §02. The cookie is written first and always, because the auth
 * and public shells have no session to write to and the choice has to survive
 * the next request either way. When there *is* a session the account is told
 * as well, so the choice follows the person to their other machines.
 *
 * The account write is best-effort on purpose: losing it costs the other
 * devices, not this page, and a language that failed to switch because a
 * profile call fell over would be a strange thing to explain.
 */

/** A year. A language preference is not a session. */
const A_YEAR = 60 * 60 * 24 * 365;

export const POST: RequestHandler = async (event) => {
	const body = (await event.request.json().catch(() => null)) as { locale?: unknown } | null;
	const locale = body?.locale;

	// It arrives from the browser, so it is checked rather than trusted — and
	// checked against what this build can speak, not what it intends to.
	if (!isLocale(locale) || !availableLocales().includes(locale)) {
		return json(
			{ error: { code: 'INVALID_LOCALE', message: 'Unsupported language' } },
			{ status: 400 }
		);
	}

	event.cookies.set(LOCALE_COOKIE, locale, {
		path: '/',
		maxAge: A_YEAR,
		httpOnly: false,
		sameSite: 'lax'
	});

	if (event.locals.user) {
		try {
			await authenticatedFetch(event, '/api/users/me', {
				method: 'PUT',
				headers: { 'content-type': 'application/json' },
				body: JSON.stringify({ locale })
			});
		} catch {
			// Best effort. The browser is switched either way.
		}
	}

	return new Response(null, { status: 204 });
};
