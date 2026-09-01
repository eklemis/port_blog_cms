import { json, error } from '@sveltejs/kit';
import { POST as backendPOST } from '$lib/shared/api/client';
import type { components } from '$lib/shared/api/v1';

type RegisterRequest = components['schemas']['CreateUserRequest'];
type RegisterSuccess =
	components['schemas']['SuccessResponse_RegisterUserResponse'];
type ErrorResponse = components['schemas']['ErrorResponse'];

export async function POST({ request }) {
	const body = (await request.json()) as RegisterRequest;

	const result = await backendPOST('/api/auth/register', { body });

	if (result.error) {
		const err = result.error as ErrorResponse;
		throw error(result.response.status, err.error.message);
	}

	const payload = (result.data as RegisterSuccess).data;

	return json(payload);
}
