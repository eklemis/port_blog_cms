import type { RequestEvent } from '@sveltejs/kit';
import { backendBaseUrl } from '$lib/shared/config/backend';
import {
	ACCESS_COOKIE,
	REFRESH_COOKIE,
	setAuthCookies,
	clearAuthCookies
} from '$lib/shared/auth/cookies.server';

async function callBackend(
	event: RequestEvent,
	path: string,
	init: RequestInit = {}
) {
	const access = event.cookies.get(ACCESS_COOKIE);

	const res = await fetch(`${backendBaseUrl}${path}`, {
		...init,
		headers: {
			...(init.headers || {}),
			Authorization: access ? `Bearer ${access}` : ''
		}
	});

	return res;
}

async function refreshToken(event: RequestEvent) {
	const refresh = event.cookies.get(REFRESH_COOKIE);
	if (!refresh) return false;

	const res = await fetch(`${backendBaseUrl}/api/auth/refresh`, {
		method: 'POST',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify({ refresh_token: refresh })
	});

	if (!res.ok) {
		clearAuthCookies(event.cookies);
		return false;
	}

	const json = await res.json();

	setAuthCookies(event.cookies, {
		access_token: json.data.access_token,
		refresh_token: json.data.refresh_token
	});

	return true;
}

export async function authenticatedFetch(
	event: RequestEvent,
	path: string,
	init: RequestInit = {}
) {
	let res = await callBackend(event, path, init);

	// If access expired → try refresh once
	if (res.status === 401) {
		const refreshed = await refreshToken(event);

		if (!refreshed) return res;

		res = await callBackend(event, path, init);
	}

	return res;
}
