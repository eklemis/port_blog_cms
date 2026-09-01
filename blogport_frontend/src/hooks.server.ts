import type { Handle } from '@sveltejs/kit';
import { authenticatedFetch } from '$lib/shared/api/backend.server';
import { clearAuthCookies } from '$lib/shared/auth/cookies.server';

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

	return resolve(event);
};
