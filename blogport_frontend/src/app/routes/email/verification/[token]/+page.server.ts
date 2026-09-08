import type { PageServerLoad } from './$types';
import { GET as backendGET } from '$lib/shared/api/client';
import { UNEXPECTED } from '$lib/shared/lib/api-failure';

/**
 * `/email/verification/[token]` — where the emailed link lands.
 *
 * Verification is a plain GET with the token in the path, so this route calls
 * the API itself: there is no token to store, no session to establish, and
 * nothing for the browser to do. It runs on the server so the link works with
 * no JavaScript, which matters for a link opened from a mail client.
 *
 * The backend answers 200 whether the address was just verified or had been
 * verified already, so those two outcomes are one branch here. That satisfies
 * "already verified is a success" and, unavoidably, gives them the same words —
 * the wire carries nothing to tell them apart.
 */

/**
 * Console Blueprint J1, the INVALID_TOKEN · TOKEN_EXPIRED branch.
 *
 * Not exported: SvelteKit validates the exports of a `+page.server.ts` and
 * rejects anything outside its list, so a stray named export is a 500 on the
 * route rather than a type error. Vitest imports this module directly and never
 * sees that check.
 */
const LINK_DEAD = 'This link has expired.';

/**
 * Every way the link can be past using. Not distinguished from one another, so
 * that nothing leaks about whether a token ever existed.
 */
const DEAD = new Set(['TOKEN_EXPIRED', 'TOKEN_INVALID', 'INVALID_TOKEN', 'USER_NOT_FOUND']);

export const load: PageServerLoad = async ({ params, locals }) => {
	// Only decides where "next" points. Asking for a new link no longer needs a
	// session: the screen asks for the address instead.
	const hasSession = Boolean(locals.user);

	const result = await backendGET('/api/auth/email-verification/{token}', {
		params: { path: { token: params.token } }
	}).catch(() => null);

	if (!result) return { verified: false, message: UNEXPECTED, dead: false, hasSession };

	if (!result.error) return { verified: true, dead: false, hasSession };

	const code = (result.error as { error?: { code?: string } })?.error?.code;
	const dead = DEAD.has(code ?? '');

	// A 500 is ours and must not be dressed up as a stale link — telling someone
	// their link expired when it did not sends them to ask for another one that
	// will fail the same way.
	return { verified: false, message: dead ? LINK_DEAD : UNEXPECTED, dead, hasSession };
};
