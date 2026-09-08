/**
 * When something last happened, said the way people say it.
 *
 * Shared rather than owned by whichever list needed it first: the posts table
 * calls this column "Updated" and the application tracker calls it "Applied".
 * Two entities may not import each other, so the one behaviour lives below both.
 */

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
export function relativeDate(iso: string | null | undefined, now: Date = new Date()): string {
	if (!iso) return '—';

	const when = new Date(iso);
	if (Number.isNaN(when.getTime())) return '—';

	const elapsed = now.getTime() - when.getTime();
	const relative = new Intl.RelativeTimeFormat(undefined, { numeric: 'auto' });

	if (elapsed < HOUR) return relative.format(-Math.round(elapsed / MINUTE), 'minute');
	if (elapsed < DAY) return relative.format(-Math.round(elapsed / HOUR), 'hour');
	if (elapsed < WEEK) return relative.format(-Math.round(elapsed / DAY), 'day');
	if (elapsed < 5 * WEEK) return relative.format(-Math.round(elapsed / WEEK), 'week');

	return new Intl.DateTimeFormat(undefined, { month: 'short', year: 'numeric' }).format(when);
}
