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

/**
 * The title rules, which are the server's.
 *
 * `create_blog_post_service` trims, refuses empty, and caps at 200 on
 * `chars().count()` — code points, so an emoji is one character and not two.
 * The code it answers with is `INVALID_TITLE` in both cases; `EMPTY_TITLE`
 * belongs to topics and blog never sends it, whatever J4's branch list says.
 */
export const TITLE_MAX = 200;

/** J4: a live counter from here on, rather than a rejection at submit. */
export const TITLE_COUNTER_FROM = 160;

export const TITLE_REQUIRED = 'A title is required.';
export const TITLE_TOO_LONG = `Titles are ${TITLE_MAX} characters at most.`;

/** `undefined` when it is usable — the shape every field validator here takes. */
export function titleError(title: string): string | undefined {
	const trimmed = title.trim();

	if (!trimmed) return TITLE_REQUIRED;
	if ([...trimmed].length > TITLE_MAX) return TITLE_TOO_LONG;

	return undefined;
}

/**
 * The body rule, which is also the server's: `validate_content` refuses a body
 * that is empty once trimmed, so a genuinely blank draft cannot be created —
 * whatever J4's "create the draft first" implies.
 */
export const CONTENT_REQUIRED = 'The post needs something in it.';

export function contentError(content: string): string | undefined {
	return content.trim() ? undefined : CONTENT_REQUIRED;
}
