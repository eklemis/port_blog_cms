import { expect, test } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { expectNoA11yViolations } from '$lib/shared/test/a11y';
import PublicPostsPage from './public-posts-page.svelte';

/** Screen / Public author index 72:66 · Mobile / Public writing 88:2163. */

/** These specs render without the stylesheet, so hit areas are not real. */
const UNSTYLED_GEOMETRY = { rules: { 'target-size': { enabled: false } } };

const props = (over: Record<string, unknown> = {}) => ({
	author: {
		username: 'janedoe',
		fullName: 'Jane Doe',
		bio: 'Backend engineer, mostly Rust.',
		avatarSrc: 'https://api.example.test/api/public/media/av-1/thumbnail'
	},
	posts: [
		{
			slug: 'building-a-cms-in-rust',
			title: 'Building a CMS in Rust',
			excerpt: 'A walk through the hexagonal layout, and the one rule the build does not enforce.',
			published: '14 Aug 2026',
			publishedAt: '2026-08-14T09:30:00Z'
		},
		{
			slug: 'notes-on-seaorm-migrations',
			title: 'Notes on SeaORM migrations',
			excerpt: 'Additive only, and why dropping a column breaks production in the gap.',
			published: '2 Aug 2026',
			publishedAt: '2026-08-02T09:30:00Z'
		}
	],
	total: 12,
	page: 1,
	perPage: 3,
	filter: null,
	...over
});

test('introduces the author, then lists what they have written', async () => {
	const screen = render(PublicPostsPage, props());

	await expect.element(screen.getByText('Backend engineer, mostly Rust.')).toBeInTheDocument();
	await expect
		.element(screen.getByRole('link', { name: 'Building a CMS in Rust' }))
		.toHaveAttribute('href', '/janedoe/blog/building-a-cms-in-rust');
	await expect.element(screen.getByText('14 Aug 2026')).toBeInTheDocument();
	await expect
		.element(
			screen.getByText('Additive only, and why dropping a column breaks production in the gap.')
		)
		.toBeInTheDocument();
});

test('names the page for the width that cannot show the author block', async () => {
	// Desktop leads with "Jane Doe" (72:84); mobile drops the author block and
	// leads with "Writing" (88:2170), because the header bar already says whose
	// page this is. One heading, and only one of its two words is ever drawn.
	const screen = render(PublicPostsPage, props());

	await expect.element(screen.getByTestId('identity')).toHaveTextContent('Jane Doe');
	await expect.element(screen.getByTestId('section')).toHaveTextContent('Writing');
});

test('an author who has written nothing gets a page, not a blank', async () => {
	const screen = render(PublicPostsPage, props({ posts: [], total: 0 }));

	await expect.element(screen.getByText('Nothing published yet.')).toBeInTheDocument();
});

test('a filter with no posts under it offers the way back out', async () => {
	const screen = render(
		PublicPostsPage,
		props({ posts: [], total: 0, filter: { id: 't-rust', title: 'Rust' } })
	);

	await expect.element(screen.getByText('No posts under this topic.')).toBeInTheDocument();
	await expect
		.element(screen.getByRole('link', { name: 'Show all posts' }))
		.toHaveAttribute('href', '/janedoe/blog');
});

test('the active filter says what it is and how to drop it', async () => {
	// 72:90 — the filtered chip carries a ×, and it is the way back to unfiltered.
	const screen = render(PublicPostsPage, props({ filter: { id: 't-rust', title: 'Rust' } }));

	await expect
		.element(screen.getByRole('link', { name: 'Clear the Rust filter' }))
		.toHaveAttribute('href', '/janedoe/blog');
});

test('no filter drawn when there is none to clear', async () => {
	const screen = render(PublicPostsPage, props());

	expect(screen.getByText('Filter', { exact: true }).elements()).toHaveLength(0);
});

test('a page of twelve can be paged through', async () => {
	const asked: number[] = [];
	const screen = render(PublicPostsPage, props({ onpage: (n: number) => asked.push(n) }));

	await expect.element(screen.getByText('2 of 12')).toBeInTheDocument();
	await screen.getByRole('button', { name: 'Next page' }).click();

	expect(asked).toEqual([2]);
});

test('never leaks the console to a reader', async () => {
	const screen = render(PublicPostsPage, props());

	expect(screen.container.innerHTML).not.toMatch(/sign in|\/studio|draft/i);
});

test('has no accessibility violations', async () => {
	render(PublicPostsPage, props({ filter: { id: 't-rust', title: 'Rust' } }));

	await expectNoA11yViolations(document.body, UNSTYLED_GEOMETRY);
});
