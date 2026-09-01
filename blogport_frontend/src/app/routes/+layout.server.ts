import type { LayoutServerLoad } from './$types';

export const load: LayoutServerLoad = async ({ fetch }) => {
	const res = await fetch('/api/users/me');

	if (!res.ok) {
		return { user: null };
	}

	const user = await res.json();
	return { user };
};
