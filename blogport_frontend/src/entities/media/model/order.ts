/**
 * Reordering a gallery.
 *
 * `position` is "display order within the role, starting at 0", and
 * `PatchMediaRequest` carries it — which is the whole reason this is possible:
 * "a gallery could not be reordered without re-uploading every image."
 *
 * The arithmetic is here rather than in the component for two reasons. It is
 * the part that is quietly easy to get wrong at the ends, and a drag and a
 * keypress have to produce the same result — §07 of the Accessibility Spec is
 * why there are two gestures at all, since a reorder that only works by
 * dragging is a reorder some people cannot do.
 */

/** The least a row needs to be ordered. */
type Ordered = { media_id: string };

/**
 * The list with the item at `from` moved to `to`.
 *
 * A copy, never in place: the rail keeps rendering the old order until the
 * server has accepted the new one, and mutating the caller's array would make
 * it lie for the length of the request.
 *
 * `to` is clamped rather than rejected. The keyboard controls are disabled at
 * the ends, but a drag can be released above the first row or below the last,
 * and clamping is kinder than discarding the gesture.
 */
export function moved<T extends Ordered>(rows: readonly T[], from: number, to: number): T[] {
	const next = [...rows];
	const item = next[from];

	if (!item) return next;

	const target = Math.max(0, Math.min(next.length - 1, to));

	next.splice(from, 1);
	next.splice(target, 0, item);

	return next;
}

/**
 * Which rows need a request, and what to tell the server.
 *
 * Every row is its own PATCH, so sending the whole list when one item moved is
 * a pile of requests that can each fail for nothing. Moving the top item down
 * one changes two positions; moving it to the end changes all of them.
 */
export function repositioned<T extends Ordered>(
	before: readonly T[],
	after: readonly T[]
): { media_id: string; position: number }[] {
	const was = new Map(before.map((row, index) => [row.media_id, index]));

	return after
		.map((row, position) => ({ media_id: row.media_id, position }))
		.filter((row) => was.get(row.media_id) !== row.position);
}
