import { json, error } from '@sveltejs/kit';
import { POST as backendPOST } from '$lib/shared/api/client';
import type { components } from '$lib/shared/api/v1';
import { setAuthCookies } from '$lib/shared/auth/cookies.server';

type LoginRequestDto = components['schemas']['LoginRequestDto'];
type ErrorResponse = components['schemas']['ErrorResponse'];

type LoginSuccess = {
	data: components['schemas']['LoginResponse'];
	success: true;
};

export async function POST({ request, cookies }) {
	const body = (await request.json()) as LoginRequestDto;

	const result = await backendPOST('/api/auth/login', { body });

	if (result.error) {
		const err = result.error as ErrorResponse;
		throw error(result.response.status, err.error.message);
	}

	const payload = (result.data as LoginSuccess).data;

	setAuthCookies(cookies, {
		access_token: payload.access_token,
		refresh_token: payload.refresh_token
	});

	return json({ user: payload.user });
}
