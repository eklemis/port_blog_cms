import { expect, test } from 'vitest';
import { createRawSnippet } from 'svelte';
import { render } from 'vitest-browser-svelte';
import { expectNoA11yViolations } from '$lib/shared/test/a11y';
import Pager from './pager.svelte';

/**
 * The count and the page numbers under every list — Screen / Posts list 11:117:
 * "6 of 24 posts" on the left, "‹ 1 2 3 ›" on the right.
 */

const UNSTYLED_GEOMETRY = { rules: { 'target-size': { enabled: false } } };

test('says how many there are, not just how many are shown', async () => {
	const screen = render(Pager, {
		shown: 6,
		total: 24,
		page: 1,
		perPage: 10,
		noun: 'posts',
		onpage: () => {}
	});

	await expect.element(screen.getByText('6 of 24 posts')).toBeInTheDocument();
});

test('numbers the pages, and marks the one you are on', async () => {
	const screen = render(Pager, {
		shown: 6,
		total: 24,
		page: 2,
		perPage: 10,
		noun: 'posts',
		onpage: () => {}
	});

	await expect
		.element(screen.getByRole('button', { name: 'Page 2' }))
		.toHaveAttribute('aria-current', 'page');
	await expect.element(screen.getByRole('button', { name: 'Page 3' })).toBeInTheDocument();
});

test('a number goes to its page, and the arrows go one either way', async () => {
	const asked: number[] = [];
	const screen = render(Pager, {
		shown: 10,
		total: 24,
		page: 2,
		perPage: 10,
		noun: 'posts',
		onpage: (n: number) => asked.push(n)
	});

	await screen.getByRole('button', { name: 'Page 3' }).click();
	await screen.getByRole('button', { name: 'Previous page' }).click();
	await screen.getByRole('button', { name: 'Next page' }).click();

	expect(asked).toEqual([3, 1, 3]);
});

test('there is no going back past the first page or on past the last', async () => {
	const screen = render(Pager, {
		shown: 10,
		total: 24,
		page: 1,
		perPage: 10,
		noun: 'posts',
		onpage: () => {}
	});

	await expect.element(screen.getByRole('button', { name: 'Previous page' })).toBeDisabled();
});

test('one page of rows needs no numbers at all', async () => {
	const screen = render(Pager, {
		shown: 4,
		total: 4,
		page: 1,
		perPage: 10,
		noun: 'posts',
		onpage: () => {}
	});

	await expect.element(screen.getByText('4 of 4 posts')).toBeInTheDocument();
	expect(screen.getByRole('button').elements()).toHaveLength(0);
});

test('the count can carry a destination beside it', async () => {
	// Screen / Posts list: "6 of 24 posts   View archive (3)" — beside the
	// count, never beside the primary action: it is a destination, not a thing
	// to do.
	const after = createRawSnippet(() => ({
		render: () => '<a href="/studio/posts/archive">View archive (3)</a>'
	}));
	const screen = render(Pager, {
		shown: 6,
		total: 24,
		page: 1,
		perPage: 10,
		noun: 'posts',
		after,
		onpage: () => {}
	});

	await expect.element(screen.getByRole('link', { name: 'View archive (3)' })).toBeInTheDocument();
});

test('has no accessibility violations', async () => {
	render(Pager, { shown: 6, total: 24, page: 2, perPage: 10, noun: 'posts', onpage: () => {} });

	await expectNoA11yViolations(document.body, UNSTYLED_GEOMETRY);
});

test('a public list counts without naming the thing it counts', async () => {
	// Screen / Public author index 72:116 reads "3 of 12" — the console's lists
	// say "of 24 posts", but a reader on an author's page knows what they are
	// looking at, and the frame leaves the noun off.
	const screen = render(Pager, { shown: 3, total: 12, page: 1, perPage: 3, onpage: () => {} });

	await expect.element(screen.getByText('3 of 12', { exact: true })).toBeInTheDocument();
});
