/**
 * The routes more than one slice needs to name.
 *
 * They live here rather than in a feature because slices in the same layer may
 * not import each other: sign-in decides where a session lands, and the hold
 * screen decides who it lets go — both need the same two destinations, and
 * neither may reach into the other for them.
 *
 * Console Blueprint §03, surface map.
 */

/** Overview. The console's front door, and only for a verified account. */
export const CONSOLE_ROUTE = '/studio';

/** The hold screen: what verification unlocks, the address it went to, a resend. */
export const HOLD_ROUTE = '/verify';

export const SIGN_IN_ROUTE = '/auth/login';

/**
 * The console's surfaces, in the order the sidebar lists them.
 *
 * Seven. It shipped with six: "Applications" was in every Overview frame and
 * had screens of its own, but the Console Blueprint's route map did not carry
 * the Career Studio at all, so there was no route to point it at and it was
 * omitted rather than invented. The map now carries it, and the Prototype Map's
 * sidebar row gives the order — Applications between Résumés and Media.
 */
export const CONSOLE_ROUTES = {
	overview: '/studio',
	posts: '/studio/posts',
	projects: '/studio/projects',
	resumes: '/studio/resumes',
	applications: '/studio/applications',
	media: '/studio/media',
	topics: '/studio/topics',
	account: '/studio/account'
} as const;
