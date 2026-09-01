import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { backendBaseUrl } from '$lib/shared/config/backend';
import { ACCESS_COOKIE } from '$lib/shared/auth/cookies.server';

export const GET: RequestHandler = async ({ fetch, cookies }) => {
	const accessToken = cookies.get(ACCESS_COOKIE);

	if (!accessToken) {
		return json({ error: 'Missing access token' }, { status: 401 });
	}

	const res = await fetch(`${backendBaseUrl}/api/users/me`, {
		method: 'GET',
		headers: {
			accept: 'application/json',
			authorization: `Bearer ${accessToken}`
		}
	});

	if (!res.ok) {
		const body = await res.json().catch(() => null);
		return json(body ?? { error: 'Unauthorized' }, { status: res.status });
	}

	const body = await res.json();

	// Your backend returns:
	// { success: true, data: { email, full_name, user_id, username } }

	return json(body.data);
};

export const PUT: RequestHandler = async ({ fetch, cookies, request }) => {
	const accessToken = cookies.get(ACCESS_COOKIE);
	if (!accessToken) return json({ error: { message: 'Missing access token' } }, { status: 401 });

	const payload = (await request.json().catch(() => null)) as { full_name?: string } | null;

	if (!payload?.full_name) {
		return json({ error: { message: 'full_name is required' } }, { status: 400 });
	}

	const res = await fetch(`${backendBaseUrl}/api/users/me`, {
		method: 'PUT',
		headers: {
			accept: 'application/json',
			'content-type': 'application/json',
			authorization: `Bearer ${accessToken}`
		},
		body: JSON.stringify({ full_name: payload.full_name })
	});

	const body = await res.json().catch(() => null);

	if (!res.ok) return json(body ?? { error: { message: 'Update failed' } }, { status: res.status });

	// backend: { success:true, data:{...} }
	return json(body.data);
};
