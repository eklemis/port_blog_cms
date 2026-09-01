import { json } from '@sveltejs/kit';
import { POST as backendPOST } from '$lib/shared/api/client';
import type { components } from '$lib/shared/api/v1';
import {
	clearAuthCookies,
	REFRESH_COOKIE
} from '$lib/shared/auth/cookies.server';

type LogoutRequestDto = components['schemas']['LogoutRequestDto'];

export async function POST({ cookies }) {
	const refresh = cookies.get(REFRESH_COOKIE);

	const body: LogoutRequestDto = {
		refresh_token: refresh ?? null
	};

	await backendPOST('/api/auth/logout', { body }).catch(() => {});

	clearAuthCookies(cookies);

	return json({ ok: true });
}
