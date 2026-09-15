import { expect, test } from 'vitest';
import { pageWindow } from './pages';

/**
 * Which page numbers the pager shows — Screen / Posts list 11:117 draws
 * "‹ 1 2 3 ›". Every page while there are few; the ends and the neighbourhood
 * of the current page once there are many, so the bar never wraps.
 */

test('a few pages are all shown', () => {
	expect(pageWindow(1, 3)).toEqual([1, 2, 3]);
	expect(pageWindow(4, 7)).toEqual([1, 2, 3, 4, 5, 6, 7]);
});

test('many pages keep the ends and the current page’s neighbours', () => {
	expect(pageWindow(10, 20)).toEqual([1, '…', 9, 10, 11, '…', 20]);
});

test('near an end, the gap is only on the far side', () => {
	expect(pageWindow(2, 20)).toEqual([1, 2, 3, '…', 20]);
	expect(pageWindow(19, 20)).toEqual([1, '…', 18, 19, 20]);
});

test('a single page is still a page', () => {
	expect(pageWindow(1, 1)).toEqual([1]);
});
