import { expect, test } from 'vitest';
import { moved, repositioned } from './order';

/**
 * Reordering a gallery.
 *
 * `PatchMediaRequest` carries `position` beside `alt_text` and `caption`, which
 * is what makes this possible at all — the endpoint's own words: "a gallery
 * could not be reordered without re-uploading every image."
 *
 * The arithmetic lives here rather than in the component because it is the part
 * that is easy to get subtly wrong, and because a drag and a keypress must
 * produce exactly the same result. §07 of the Accessibility Spec is the reason
 * there are two gestures: a reorder that only works by dragging is a reorder
 * some people cannot do.
 */

const ids = (rows: { media_id: string }[]) => rows.map((row) => row.media_id);

const gallery = () => [{ media_id: 'a' }, { media_id: 'b' }, { media_id: 'c' }];

test('moving an item down puts it after its neighbour', () => {
	expect(ids(moved(gallery(), 0, 1))).toEqual(['b', 'a', 'c']);
});

test('moving an item up puts it before its neighbour', () => {
	expect(ids(moved(gallery(), 2, 1))).toEqual(['a', 'c', 'b']);
});

test('a drag across the list carries the item the whole way', () => {
	expect(ids(moved(gallery(), 0, 2))).toEqual(['b', 'c', 'a']);
});

test('the first item cannot move up, and nothing is lost trying', () => {
	// The keyboard control is disabled at the ends, but a drag can still be
	// dropped above the top. Clamping beats discarding.
	expect(ids(moved(gallery(), 0, -1))).toEqual(['a', 'b', 'c']);
	expect(ids(moved(gallery(), 2, 9))).toEqual(['a', 'b', 'c']);
});

test('moving an item onto itself changes nothing', () => {
	expect(ids(moved(gallery(), 1, 1))).toEqual(['a', 'b', 'c']);
});

test('the original list is not rearranged under the caller', () => {
	// The component renders the old order until the server has taken the new
	// one; mutating in place would make the rail lie during the request.
	const before = gallery();

	moved(before, 0, 2);

	expect(ids(before)).toEqual(['a', 'b', 'c']);
});

test('only the rows whose position actually changed are worth a request', () => {
	// Each is its own PATCH. Sending three when one moved is two requests that
	// can fail for nothing.
	const after = moved(gallery(), 0, 1);

	expect(repositioned(gallery(), after)).toEqual([
		{ media_id: 'b', position: 0 },
		{ media_id: 'a', position: 1 }
	]);
});

test('a list that did not move asks for nothing', () => {
	expect(repositioned(gallery(), gallery())).toEqual([]);
});

test('position is the index, so the server and the screen agree', () => {
	// `MediaItem.position` is "display order within the role, starting at 0".
	const after = moved(gallery(), 2, 0);

	expect(repositioned(gallery(), after)).toEqual([
		{ media_id: 'c', position: 0 },
		{ media_id: 'a', position: 1 },
		{ media_id: 'b', position: 2 }
	]);
});
