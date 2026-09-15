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

/**
 * Where a published post lives.
 *
 * `/[username]/blog/[slug]`, which is the row in §03's public surface map. J4's
 * prose shortens it to `/{username}/{slug}` — that is not a route in the map,
 * and the map is the contract.
 *
 * Both halves are escaped: a username is chosen by a person and a slug accepts
 * anything the server accepts, which includes spaces.
 */
export function publicPostPath(username: string, slug: string): string {
	return `/${encodeURIComponent(username)}/blog/${encodeURIComponent(slug)}`;
}

const HOUR_MS = 3_600_000;
const DAY_MS = 24 * HOUR_MS;
const WEEK_MS = 7 * DAY_MS;

/**
 * When a scheduled post goes live, as Mobile / Posts list 73:31 says it:
 * "goes live tomorrow", "goes live in 3 hours", and past five weeks the month
 * — "goes live in Jan 2027" — because a countdown in weeks stops meaning much.
 *
 * Only for a date in the future. A post whose date has passed is live, and
 * says so through `postStatus`.
 */
export function scheduledLabel(publishedAt: string, now: Date = new Date()): string {
	const when = new Date(publishedAt);
	const ahead = when.getTime() - now.getTime();
	const relative = new Intl.RelativeTimeFormat(undefined, { numeric: 'auto' });

	// Days by the calendar, not by 24-hour blocks: nine tomorrow morning is
	// "tomorrow" even when it is only fourteen hours away.
	const midnight = (date: Date) =>
		new Date(date.getFullYear(), date.getMonth(), date.getDate()).getTime();
	const days = Math.round((midnight(when) - midnight(now)) / DAY_MS);

	if (days === 0)
		return `goes live ${relative.format(Math.max(1, Math.round(ahead / HOUR_MS)), 'hour')}`;
	if (days < 7) return `goes live ${relative.format(days, 'day')}`;
	if (ahead < 5 * WEEK_MS)
		return `goes live ${relative.format(Math.round(ahead / WEEK_MS), 'week')}`;

	const month = new Intl.DateTimeFormat(undefined, { month: 'short', year: 'numeric' }).format(
		when
	);
	return `goes live in ${month}`;
}
