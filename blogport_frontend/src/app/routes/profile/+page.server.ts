import type { PageServerLoad } from './$types';
import { redirect } from '@sveltejs/kit';

export const load: PageServerLoad = async ({ fetch }) => {
  const res = await fetch('/api/users/me');

	if (res.status === 401) {
		throw redirect(302, '/auth/login');
	}

	if (!res.ok) {
		return { user: null, error: 'Failed to load profile.' };
	}

	const user = await res.json();
	return { user, error: null };
};
