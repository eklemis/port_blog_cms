import { relativeDate } from '$lib/shared/lib/relative-time';

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

/**
 * When the post last changed: "5 hours ago", "3 weeks ago", "Apr 2026".
 *
 * The shared helper under another name. The application tracker needs the same
 * sentence for its "Applied" column, and one entity may not reach into another
 * for it.
 */
export const updatedLabel = relativeDate;
