import type { Cookies } from '@sveltejs/kit';

const isProd = process.env.NODE_ENV === 'production';

export const ACCESS_COOKIE = 'access_token';
export const REFRESH_COOKIE = 'refresh_token';

/**
 * The address a verification link was just sent to.
 *
 * Registration returns a user and no token, so there is no session — and J1
 * still says route straight to /verify "showing the address the mail went to".
 * This is where that address lives between the two requests. A query parameter
 * would put someone's email in their history and in every referrer; a cookie
 * keeps it server-side and expires on its own.
 *
 * It lasts as long as the link does (JWT_VERIFICATION_EXPIRY, 24h), and a real
 * session always outranks it.
 */
export const PENDING_EMAIL_COOKIE = 'pending_verification_email';

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

const PENDING_EMAIL_MAX_AGE = 60 * 60 * 24; // 86400s — JWT_VERIFICATION_EXPIRY

export function setPendingVerificationEmail(cookies: Cookies, email: string) {
	cookies.set(PENDING_EMAIL_COOKIE, email, {
		httpOnly: true,
		secure: isProd,
		sameSite: 'lax',
		path: '/',
		maxAge: PENDING_EMAIL_MAX_AGE
	});
}

export function clearPendingVerificationEmail(cookies: Cookies) {
	cookies.delete(PENDING_EMAIL_COOKIE, { path: '/' });
}

export function clearAuthCookies(cookies: Cookies) {
	cookies.delete(ACCESS_COOKIE, { path: '/' });
	cookies.delete(REFRESH_COOKIE, { path: '/' });
}
