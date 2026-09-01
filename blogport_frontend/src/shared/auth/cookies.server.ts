import type { Cookies } from '@sveltejs/kit';

const isProd = process.env.NODE_ENV === 'production';

export const ACCESS_COOKIE = 'access_token';
export const REFRESH_COOKIE = 'refresh_token';

export function setAuthCookies(
	cookies: Cookies,
	tokens: { access_token: string; refresh_token: string }
) {
	cookies.set(ACCESS_COOKIE, tokens.access_token, {
		httpOnly: true,
		secure: isProd,
		sameSite: 'lax',
		path: '/',
		maxAge: 60 * 15 // 15 minutes
	});

	cookies.set(REFRESH_COOKIE, tokens.refresh_token, {
		httpOnly: true,
		secure: isProd,
		sameSite: 'lax',
		path: '/',
		maxAge: 60 * 60 * 24 * 14 // 14 days
	});
}

export function clearAuthCookies(cookies: Cookies) {
	cookies.delete(ACCESS_COOKIE, { path: '/' });
	cookies.delete(REFRESH_COOKIE, { path: '/' });
}
