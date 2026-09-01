// See https://svelte.dev/docs/kit/types#app.d.ts
// for information about these interfaces
declare global {
	namespace App {
		// interface Error {}
		interface Locals {
				user: {
					email: string;
					full_name: string;
					user_id: string;
					username: string;
				} | null;
			}
		// interface PageData {}
		// interface PageState {}
		// interface Platform {}
	}
}

export {};
