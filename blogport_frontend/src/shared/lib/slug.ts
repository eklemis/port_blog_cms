/**
 * The public address a post or a project will live at.
 *
 * Shared rather than owned by posts: J5 says projects handle slugs "the same
 * as posts", and two entities may not import each other.
 *
 * The rules are the server's, from VALIDATION.md: trimmed, lowercased,
 * non-empty, at most 200 characters. Nothing else — the API accepts spaces and
 * punctuation in a slug, so refusing them here would refuse an address the
 * backend would have taken. Deriving a good one is this module's job;
 * overruling the person who types their own is not.
 */

/** Rust counts `chars()`, so an emoji is one character and not two. */
const length = (value: string) => [...value].length;

export const SLUG_MAX = 200;

export const SLUG_REQUIRED = 'A web address is required.';
export const SLUG_TOO_LONG = `Web addresses are ${SLUG_MAX} characters at most.`;

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

	return undefined;
}
