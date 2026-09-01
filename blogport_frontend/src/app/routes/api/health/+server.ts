import { json } from '@sveltejs/kit';
import { GET } from '$lib/shared/api/client';

export const GET as GET_handler = async () => {
	const r = await GET('/health');
	// adjust path to whatever your backend exposes
	return json({
		ok: r.error === undefined,
		status: r.response.status,
		error: r.error ?? null
	});
};
