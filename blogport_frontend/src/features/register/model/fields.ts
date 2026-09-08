/**
 * The two rules register adds. Email and password are shared with sign-in and
 * live in `$lib/shared/lib`.
 *
 * Mirrored from Frontend Handoff §03 and no further.
 */

/** Forms & Interaction Spec §03, the Register table. */
export const USERNAME_INVALID = 'Letters, numbers and underscores only.';
export const FULL_NAME_REQUIRED = 'Please add your name.';

/**
 * Not in the copy table, which gives one sentence for username and it is about
 * the character set. Phrased to match the password's minimum, which is the only
 * other floor a person meets on this form. See the PR.
 */
export const USERNAME_TOO_SHORT = 'At least 3 characters.';

export const USERNAME_MIN = 3;
export const USERNAME_MAX = 50;
export const FULL_NAME_MAX = 100;

const ALLOWED = /^[A-Za-z0-9_]+$/;

/**
 * Trimmed and lowercased, which is what the server stores. Case is not the
 * person's mistake: `JaneDoe` is accepted and becomes `janedoe`.
 */
export function normaliseUsername(raw: string): string {
	return raw.trim().toLowerCase();
}

export function usernameError(raw: string): string | undefined {
	const username = normaliseUsername(raw);

	// Length first: "Letters, numbers and underscores only" explains nothing to
	// someone whose problem is that they typed two of them.
	if (username.length < USERNAME_MIN) return USERNAME_TOO_SHORT;

	return ALLOWED.test(username) ? undefined : USERNAME_INVALID;
}

/**
 * The address this username becomes, or nothing yet.
 *
 * Shown under the field while they type, because it is permanent and it is
 * lowercased on the way in — someone typing `JaneDoe` should see the address
 * they are actually getting before they commit to it, not discover it after.
 * The field itself is left alone: rewriting what someone is typing is worse
 * than telling them what it will mean.
 */
export function publicAddress(raw: string): string | undefined {
	const username = normaliseUsername(raw);
	return username ? `/${username}` : undefined;
}

/** Non-empty once trimmed. Not a format — names are not a format. */
export function fullNameError(raw: string): string | undefined {
	return raw.trim().length === 0 ? FULL_NAME_REQUIRED : undefined;
}
