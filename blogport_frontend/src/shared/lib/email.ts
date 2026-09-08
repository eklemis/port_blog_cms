/**
 * What counts as an email address on the way in.
 *
 * In `shared` because more than one slice asks the question — sign-in and the
 * expired-link screen both do — and slices in the same layer may not import
 * each other.
 *
 * Deliberately loose. The server is the authority on what it accepts, so this
 * only has to catch what is obviously not an address yet: the cost of a false
 * rejection is someone who cannot get in at all, and the cost of a false
 * acceptance is one round trip.
 */

/** Register table, Forms & Interaction Spec §03. Same field, same sentence. */
export const EMAIL_INVALID = 'That doesn’t look like an email address.';

const LOOKS_LIKE_AN_ADDRESS = /^[^\s@]+@[^\s@.]+(?:\.[^\s@.]+)+$/;

/** The address as it goes on the wire. The backend trims and lowercases it. */
export function normaliseEmail(raw: string): string {
	return raw.trim();
}

export function emailError(raw: string): string | undefined {
	return LOOKS_LIKE_AN_ADDRESS.test(normaliseEmail(raw)) ? undefined : EMAIL_INVALID;
}
