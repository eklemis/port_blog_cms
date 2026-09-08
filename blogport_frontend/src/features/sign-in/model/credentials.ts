/**
 * The password rule the sign-in form checks before it spends a round trip. The
 * email rule is in `$lib/shared/lib/email`, because the expired-link screen
 * asks the same question and slices may not import each other.
 *
 * Both are mirrored from Frontend Handoff §03 — the use cases and services, not
 * the OpenAPI examples — and neither goes further than the server does. A
 * client-side rule the API disagrees with rejects input the backend would have
 * accepted, which is a real bug rather than a safety net.
 */

export const PASSWORD_TOO_SHORT = 'At least 12 characters.';

/**
 * 12 ≤ length ≤ 128, and nothing else. `BasicPasswordPolicy` checks no
 * uppercase, digit or symbol requirement; the OpenAPI example value is
 * `SecurePass123!`, which implies complexity rules that do not exist. Do not
 * add them here.
 */
export const PASSWORD_MIN = 12;
export const PASSWORD_MAX = 128;

/**
 * Length only. Nothing is trimmed — normalisation for a password is "none", so
 * the spaces someone deliberately typed are part of it.
 *
 * The maximum is enforced by capping the input at 128 rather than reporting it:
 * the copy table has a sentence for the minimum and none for the maximum, and a
 * field that cannot overrun needs no message.
 */
export function passwordError(password: string): string | undefined {
	return passwordLength(password) < PASSWORD_MIN ? PASSWORD_TOO_SHORT : undefined;
}

/**
 * Characters, not UTF-16 code units. The backend measures with Rust's
 * `chars()`, so a password of twelve emoji is twelve there and twenty-four to
 * `String.prototype.length` — and one of eleven would pass this check and be
 * rejected by the server.
 */
export function passwordLength(password: string): number {
	return [...password].length;
}
