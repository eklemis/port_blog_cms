// See https://svelte.dev/docs/kit/types#app.d.ts
// for information about these interfaces
declare global {
	namespace App {
		// interface Error {}
		interface Locals {
			/**
			 * Rebuilt from `GET /api/users/me` on every request by hooks.server.ts,
			 * so this mirrors `UserProfileResponse` exactly. Note what is NOT here:
			 * `is_verified`. Only the login response carries it, which is why no
			 * page load can decide whether an account has verified.
			 */
			user: {
				user_id: string;
				email: string;
				username: string;
				full_name: string;
				bio: string | null;
				locale: string;
			} | null;
		}
		// interface PageData {}
		// interface PageState {}
		// interface Platform {}
	}
}

export {};
