import type { components } from '$lib/shared/api/v1';

type RegisterRequest = components['schemas']['CreateUserRequest'];
type LoginRequest = components['schemas']['LoginRequestDto'];

async function postJson<T>(url: string, body: unknown): Promise<T> {
	const res = await fetch(url, {
		method: 'POST',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify(body)
	});

	if (!res.ok) {
		const message = await res.text().catch(() => 'Request failed');
		throw new Error(message || 'Request failed');
	}

	return (await res.json()) as T;
}

export async function register(dto: RegisterRequest) {
	return postJson('/api/auth/register', dto);
}

export async function login(dto: LoginRequest) {
	return postJson('/api/auth/login', dto);
}

export async function logout() {
	return postJson<{ ok: boolean }>('/api/auth/logout', {});
}
