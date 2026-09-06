import type { Cookies } from '@sveltejs/kit';

const isProd = process.env.NODE_ENV === 'production';

export const ACCESS_COOKIE = 'access_token';
export const REFRESH_COOKIE = 'refresh_token';

/**
 * Cookie lifetimes must match the tokens the backend actually issues, read from
 * `jwt/jwt_config.rs`. They were previously 15 minutes and 14 days, which was
 * wrong in both directions: it threw away half of every session, and for a full
 * week the browser held a refresh cookie the server had already stopped
 * honouring — producing a user who looks signed in and is not.
 *
 * If the backend's JWT_ACCESS_EXPIRY / JWT_REFRESH_EXPIRY are overridden in an
 * environment, read `exp` off the token instead of editing these. Do not
 * hardcode a third opinion.
 */
const ACCESS_MAX_AGE = 60 * 30; // 1800s — JWT_ACCESS_EXPIRY
const REFRESH_MAX_AGE = 60 * 60 * 24 * 7; // 604800s — JWT_REFRESH_EXPIRY

export function setAuthCookies(
	cookies: Cookies,
	tokens: { access_token: string; refresh_token: string }
) {
	cookies.set(ACCESS_COOKIE, tokens.access_token, {
		httpOnly: true,
		secure: isProd,
		sameSite: 'lax',
		path: '/',
		maxAge: ACCESS_MAX_AGE
	});

	cookies.set(REFRESH_COOKIE, tokens.refresh_token, {
		httpOnly: true,
		secure: isProd,
		sameSite: 'lax',
		path: '/',
		maxAge: REFRESH_MAX_AGE
	});
}

export function clearAuthCookies(cookies: Cookies) {
	cookies.delete(ACCESS_COOKIE, { path: '/' });
	cookies.delete(REFRESH_COOKIE, { path: '/' });
}
