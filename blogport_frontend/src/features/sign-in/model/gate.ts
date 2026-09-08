/**
 * Where a successful sign-in lands.
 *
 * Login is not the gate; verification is. `POST /api/auth/login` succeeds for an
 * unverified account and returns a working access token, and every authoring
 * route then refuses that token with 403 EMAIL_NOT_VERIFIED. So the console is
 * not a destination this decision may reach without checking `is_verified` —
 * anything else shows a person a full authoring UI that fails on every button.
 *
 * Console Blueprint §02 and journey J1.
 */

import { CONSOLE_ROUTE, HOLD_ROUTE } from '$lib/shared/config/routes';

/**
 * A saved destination arrives in the URL, so it is attacker-controlled: an
 * absolute or protocol-relative URL here would turn the sign-in screen into an
 * open redirect. Only a plain same-site path survives.
 */
export function safeDestination(next: string | null | undefined): string | null {
	if (!next) return null;

	const candidate = next.trim();

	// Must be a path on this site: one leading slash, and not the "//host" or
	// "/\host" forms that browsers resolve to a different origin.
	if (!candidate.startsWith('/')) return null;
	if (candidate.startsWith('//') || candidate.startsWith('/\\')) return null;

	return candidate;
}

export function destinationAfterSignIn(
	user: { is_verified: boolean },
	next: string | null | undefined
): string {
	// Checked first and unconditionally. A `next` captured before an expiry
	// points into the console, and honouring it for an unverified account is
	// exactly the bug this function exists to prevent.
	if (!user.is_verified) return HOLD_ROUTE;

	return safeDestination(next) ?? CONSOLE_ROUTE;
}
