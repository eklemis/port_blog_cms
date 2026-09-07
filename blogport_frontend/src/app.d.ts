// See https://svelte.dev/docs/kit/types#app.d.ts
// for information about these interfaces
declare global {
	namespace App {
		// interface Error {}
		interface Locals {
			/**
			 * Rebuilt from `GET /api/users/me` on every request by hooks.server.ts,
			 * so this mirrors `UserProfileResponse` exactly. `is_verified` is read
			 * from the row rather than the token claim, which is what makes it the
			 * only fresh answer — the claim and the login response are snapshots.
			 */
			user: {
				user_id: string;
				email: string;
				username: string;
				full_name: string;
				bio: string | null;
				locale: string;
				is_verified: boolean;
			} | null;
		}
		// interface PageData {}
		// interface PageState {}
		// interface Platform {}
	}
}

export {};
