/**
 * The two things a published post says about itself above the title —
 * Screen / Public post 72:17, "14 August 2026 · 7 min read".
 */

/**
 * The publication date, in the reader's own spelling.
 *
 * `Intl` is asked for the parts and decides the order and the words. Writing
 * "14 August 2026" by hand reads correctly today and wrongly the day a second
 * locale is added, which is the failure the Career Studio paper warns about and
 * which the designer has already overruled once on the tracker.
 */
export function publishedLabel(
	publishedAt: string | null | undefined,
	locale?: string
): string | null {
	if (!publishedAt) return null;

	const on = new Date(publishedAt);
	if (Number.isNaN(on.getTime())) return null;

	return new Intl.DateTimeFormat(locale, {
		day: 'numeric',
		month: 'long',
		year: 'numeric'
	}).format(on);
}

/**
 * The same date, shorter, for a row in a list.
 *
 * Screen / Public author index 72:101 reads "14 Aug 2026" where the post page
 * reads "14 August 2026". Two formats deliberately: a list wants a date that
 * stays out of the way, and an article wants one that reads as prose. Both ask
 * `Intl` for the parts, so neither spells a month.
 */
export function listedLabel(
	publishedAt: string | null | undefined,
	locale?: string
): string | null {
	if (!publishedAt) return null;

	const on = new Date(publishedAt);
	if (Number.isNaN(on.getTime())) return null;

	return new Intl.DateTimeFormat(locale, {
		day: 'numeric',
		month: 'short',
		year: 'numeric'
	}).format(on);
}

/** Words a minute. The usual figure for prose read on a screen. */
const PER_MINUTE = 200;

/**
 * How long the body takes to read, in whole minutes.
 *
 * Counted on the words rather than the source: the marks that make Markdown are
 * not things a person reads, and an image reference is a line of syntax that
 * reads as nothing at all. Always at least one — "0 min read" describes no post
 * anybody wrote.
 */
export function readingTime(content: string): number {
	const prose = content
		// Image references and link destinations: syntax, not sentences.
		.replace(/!\[[^\]]*\]\([^)]*\)/g, ' ')
		.replace(/\[([^\]]*)\]\([^)]*\)/g, '$1')
		// Fences, inline code ticks, and the marks that decorate words.
		.replace(/```[\s\S]*?```/g, ' ')
		.replace(/[`*_>#~-]/g, ' ');

	const words = prose.split(/\s+/).filter(Boolean).length;

	return Math.max(1, Math.ceil(words / PER_MINUTE));
}
