/**
 * The public address a post or a project will live at.
 *
 * Shared rather than owned by posts: J5 says projects handle slugs "the same
 * as posts", and two entities may not import each other.
 *
 * The rules are the server's: trimmed, lowercased, non-empty, at most 200
 * characters, and `a-z`, `0-9` and `-` only. That last one was missing from
 * VALIDATION.md when this was first written, so the first cut accepted spaces
 * and punctuation on the grounds that the document did not forbid them. Both
 * the create and the patch service test
 * `is_ascii_alphanumeric() || c == '-'`, so `My Post Title` is an
 * `INVALID_SLUG` and not an address with `%20` in it.
 *
 * `is_ascii_alphanumeric` is exactly that: `café` and `日本語` are good titles
 * and impossible slugs. Deriving one folds the accents away; typing one by
 * hand has to be refused here, because it will be refused there.
 */

/** Rust counts `chars()`, so an emoji is one character and not two. */
const length = (value: string) => [...value].length;

export const SLUG_MAX = 200;

export const SLUG_REQUIRED = 'A web address is required.';
export const SLUG_TOO_LONG = `Web addresses are ${SLUG_MAX} characters at most.`;
export const SLUG_CHARACTERS = 'Use lowercase letters, numbers and hyphens only.';

/** The server's test, written as one: `is_ascii_alphanumeric() || c == '-'`. */
const ALLOWED = /^[a-z0-9-]+$/;

/**
 * A title, as the address someone would have typed for it.
 *
 * Accents are folded rather than stripped — "Résumés" has to become
 * `resumes` and not `rsums`, and the product's own navigation says Résumés.
 *
 * A title with nothing sluggable gives back an empty string rather than a bare
 * separator: the field then shows as empty and asks to be filled in, which is
 * better than an address that reads "-".
 */
export function slugFrom(title: string): string {
	const folded = title
		.normalize('NFD')
		// Combining marks, which is what NFD split the accents into.
		.replace(/[̀-ͯ]/g, '')
		.toLowerCase();

	const dashed = folded
		.replace(/[^a-z0-9]+/g, '-')
		.replace(/-+/g, '-')
		.replace(/^-|-$/g, '');

	// Cut on the cap, then again on the separator, so the address never ends
	// mid-word with a trailing dash.
	return dashed.slice(0, SLUG_MAX).replace(/-$/, '');
}

/** `undefined` when it is usable — the shape every field validator here takes. */
export function slugError(slug: string): string | undefined {
	const trimmed = slug.trim();

	if (!trimmed) return SLUG_REQUIRED;
	if (length(trimmed) > SLUG_MAX) return SLUG_TOO_LONG;
	// Lowercased on write, so an uppercase letter is not a failure — but it is
	// also not what will be stored, and saying so now beats surprising someone
	// with a different address than the one they typed.
	if (!ALLOWED.test(trimmed.toLowerCase())) return SLUG_CHARACTERS;

	return undefined;
}
