/**
 * Which page numbers the pager shows.
 *
 * Every page while there are seven or fewer. Past that, the first and last
 * pages and the current page's neighbours, with a gap marked where pages are
 * skipped — so the bar has a bounded width and never wraps under the table.
 */
export type PageSlot = number | '…';

export function pageWindow(page: number, last: number): PageSlot[] {
	if (last <= 7) return Array.from({ length: last }, (_, index) => index + 1);

	const around = [page - 1, page, page + 1].filter((n) => n > 1 && n < last);
	const slots: PageSlot[] = [1];

	if (around[0] > 2) slots.push('…');
	slots.push(...around);
	if (around[around.length - 1] < last - 1) slots.push('…');
	slots.push(last);

	return slots;
}
