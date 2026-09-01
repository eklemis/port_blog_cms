import type { PageServerLoad } from './$types';
import { backendBaseUrl } from '$lib/shared/config/backend';

function looksLikeToken(token: string) {
	// Supports JWT and URL-safe tokens. JWT has dots.
	return /^[A-Za-z0-9._-]+$/.test(token) && token.length >= 16 && token.length <= 2048;
}

export const load: PageServerLoad = async ({ fetch, params }) => {
	const token = params.token;

	if (!looksLikeToken(token)) {
		return { ok: false, message: 'Invalid verification link.' };
	}

	const res = await fetch(`${backendBaseUrl}/api/auth/email-verification/${token}`, {
		method: 'GET',
		headers: { accept: 'application/json' }
	});

	const body = (await res.json().catch(() => null)) as
		| { success: true; data: { message: string } }
		| { success: false; error: { code: string; message: string } }
		| null;

	if (res.ok && body && 'success' in body && body.success) {
		return { ok: true, message: body.data.message };
	}

	const msg =
		body && 'success' in body && body.success === false
			? body.error.message
			: 'Verification failed. The link may be invalid or expired.';

	return { ok: false, message: msg };
};
