/**
 * What a post row can say about itself, from what the list actually returns.
 *
 * `BlogPostCardResponse` carries `published_at` and no status field, so three
 * of the four states the frame draws come from that one timestamp. The fourth,
 * archived, is not derivable from anything on the card — see the PR.
 */

export type PostStatus = {
	tone: 'neutral' | 'inflight' | 'live';
	label: string;
};

/**
 * Live, scheduled or draft.
 *
 * An unparseable date is a draft rather than live: claiming something is public
 * when we cannot tell is the more expensive way to be wrong.
 */
export function postStatus(
	publishedAt: string | null | undefined,
	now: Date = new Date()
): PostStatus {
	if (!publishedAt) return { tone: 'neutral', label: 'Draft' };

	const when = new Date(publishedAt);
	if (Number.isNaN(when.getTime())) return { tone: 'neutral', label: 'Draft' };

	return when.getTime() > now.getTime()
		? { tone: 'inflight', label: 'Scheduled' }
		: { tone: 'live', label: 'Live' };
}

const MINUTE = 60_000;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;
const WEEK = 7 * DAY;

/**
 * "5 hours ago", "yesterday", "3 weeks ago", "Apr 2026".
 *
 * Relative while relative still means something, and a plain month beyond that:
 * "thirty-four weeks ago" is arithmetic rather than information. Formatted
 * through Intl so it follows the interface locale.
 */
export function updatedLabel(updatedAt: string, now: Date = new Date()): string {
	const when = new Date(updatedAt);
	if (Number.isNaN(when.getTime())) return '—';

	const elapsed = now.getTime() - when.getTime();
	const relative = new Intl.RelativeTimeFormat(undefined, { numeric: 'auto' });

	if (elapsed < HOUR) return relative.format(-Math.round(elapsed / MINUTE), 'minute');
	if (elapsed < DAY) return relative.format(-Math.round(elapsed / HOUR), 'hour');
	if (elapsed < WEEK) return relative.format(-Math.round(elapsed / DAY), 'day');
	if (elapsed < 5 * WEEK) return relative.format(-Math.round(elapsed / WEEK), 'week');

	return new Intl.DateTimeFormat(undefined, { month: 'short', year: 'numeric' }).format(when);
}
