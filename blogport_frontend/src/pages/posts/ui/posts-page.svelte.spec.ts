import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { expectNoA11yViolations } from '$lib/shared/test/a11y';
import PostsPage from './posts-page.svelte';

/**
 * The posts list, and the four states every collection ships.
 *
 * The one that earns its own tests is filtered-empty: conflating it with empty
 * tells someone with 24 posts that they have none.
 */

const UNSTYLED_GEOMETRY = { rules: { 'target-size': { enabled: false } } };

const rows = [
	{
		id: '1',
		title: 'Building a CMS in Rust',
		published_at: '2020-01-01T00:00:00Z',
		updated_at: '2026-09-06T12:00:00Z'
	},
	{
		id: '2',
		title: 'Why hexagonal, actually',
		published_at: null,
		updated_at: '2026-09-08T07:00:00Z'
	}
];

const TOPICS = [
	{ id: 'topic-1', title: 'Rust', description: '' },
	{ id: 'topic-2', title: 'Distributed Systems', description: '' }
];

const base = {
	posts: rows,
	topics: [] as typeof TOPICS,
	total: 24,
	page: 1,
	perPage: 10,
	filtered: false,
	onquery: () => {}
};

beforeEach(() => vi.useRealTimers());
afterEach(() => vi.useRealTimers());

// ── the rows ───────────────────────────────────────────────────────────────

test('lists the posts with the state each is in', async () => {
	const screen = render(PostsPage, base);

	await expect.element(screen.getByText('Building a CMS in Rust')).toBeInTheDocument();
	// Exact: "Drafts & published" and "Drafts only" are options in the filter,
	// and a substring match would find those too.
	await expect.element(screen.getByText('Live', { exact: true })).toBeInTheDocument();
	await expect.element(screen.getByText('Draft', { exact: true })).toBeInTheDocument();
});

test('says how many of how many', async () => {
	const screen = render(PostsPage, base);

	await expect.element(screen.getByText('2 of 24 posts')).toBeInTheDocument();
});

test('the update time carries its exact value for anything that needs it', async () => {
	// The relative phrase is for reading; the machine-readable one is for
	// everything else.
	render(PostsPage, base);

	const stamps = document.querySelectorAll('time[datetime]');
	expect(stamps.length).toBe(2);
});

// ── the four states ────────────────────────────────────────────────────────

test('empty says why, and offers the one action that fixes it', async () => {
	const screen = render(PostsPage, { ...base, posts: [], total: 0 });

	await expect.element(screen.getByText('No posts yet.')).toBeInTheDocument();
	await expect
		.element(screen.getByRole('link', { name: 'Write your first post' }))
		.toHaveAttribute('href', '/studio/posts/new');
});

test('filtered-empty is a different screen from empty', async () => {
	const screen = render(PostsPage, { ...base, posts: [], filtered: true });

	await expect.element(screen.getByText('No posts match those filters.')).toBeInTheDocument();
	// It says what does exist, so nobody thinks their work is gone.
	await expect.element(screen.getByText('You have 24 posts in total.')).toBeInTheDocument();
	await expect.element(screen.getByRole('button', { name: 'Clear filters' })).toBeInTheDocument();
});

test('clearing filters drops them from the query, not just the box', async () => {
	const asked: Record<string, string | null>[] = [];
	const screen = render(PostsPage, {
		...base,
		posts: [],
		filtered: true,
		search: 'rust',
		onquery: (c: Record<string, string | null>) => asked.push(c)
	});

	await screen.getByRole('button', { name: 'Clear filters' }).click();

	expect(asked[0]).toEqual({ search: null, published: null, topic_id: null, page: null });
});

// ── the topic filter ───────────────────────────────────────────────────────

test('offers the topics it was given, and every post as the default', async () => {
	const screen = render(PostsPage, { ...base, topics: TOPICS });

	await expect.element(screen.getByRole('option', { name: 'All topics' })).toBeInTheDocument();
	await expect.element(screen.getByRole('option', { name: 'Rust' })).toBeInTheDocument();
});

test('choosing a topic filters the list and returns to the first page', async () => {
	const asked: Record<string, string | null>[] = [];
	const screen = render(PostsPage, {
		...base,
		topics: TOPICS,
		onquery: (c: Record<string, string | null>) => asked.push(c)
	});

	await screen.getByRole('combobox', { name: 'Topic' }).selectOptions('topic-1');

	expect(asked).toEqual([{ topic_id: 'topic-1', page: null }]);
});

test('no topics means no control, rather than a select with nothing in it', async () => {
	// The options come from a second request. Losing it costs the control, and
	// an empty dropdown is a worse answer than none.
	const screen = render(PostsPage, { ...base, topics: [] });

	expect(screen.getByRole('combobox', { name: 'Topic' }).elements()).toHaveLength(0);
});

test('error never blames the person, and says the work is safe', async () => {
	const screen = render(PostsPage, { ...base, posts: [], failed: true });

	await expect.element(screen.getByText("We couldn't load your posts.")).toBeInTheDocument();
	await expect.element(screen.getByText(/Nothing has happened to them/)).toBeInTheDocument();
	await expect.element(screen.getByRole('button', { name: 'Try again' })).toBeInTheDocument();
});

test('loading is rows, not a spinner, and it is announced', async () => {
	const screen = render(PostsPage, { ...base, posts: [], loading: true });

	await expect.element(screen.getByRole('status')).toHaveTextContent('Loading posts');
	expect(screen.getByText('No posts yet.').elements()).toHaveLength(0);
});

// ── the control bar ────────────────────────────────────────────────────────

test('search waits for a pause rather than firing per keystroke', async () => {
	const asked: Record<string, string | null>[] = [];
	const screen = render(PostsPage, {
		...base,
		onquery: (c: Record<string, string | null>) => asked.push(c)
	});

	await screen.getByRole('searchbox', { name: 'Search posts' }).fill('rust');

	expect(asked, 'a request per character is a rate limit waiting to happen').toHaveLength(0);
	await vi.waitFor(() => expect(asked).toEqual([{ search: 'rust', page: null }]), {
		timeout: 2000
	});
});

test('searching returns to the first page', async () => {
	// Page 3 of the old results is not page 3 of the new ones.
	const asked: Record<string, string | null>[] = [];
	const screen = render(PostsPage, {
		...base,
		page: 3,
		onquery: (c: Record<string, string | null>) => asked.push(c)
	});

	await screen.getByRole('searchbox', { name: 'Search posts' }).fill('rust');

	await vi.waitFor(() => expect(asked[0]).toMatchObject({ page: null }), { timeout: 2000 });
});

test('the pager does not offer a page that is not there', async () => {
	const screen = render(PostsPage, { ...base, page: 1, total: 24 });

	await expect.element(screen.getByRole('button', { name: 'Previous' })).toBeDisabled();
	await expect.element(screen.getByRole('button', { name: 'Next' })).not.toBeDisabled();
});

test('one page of results needs no pager at all', async () => {
	const screen = render(PostsPage, { ...base, total: 2 });

	expect(screen.getByRole('button', { name: 'Next' }).elements()).toHaveLength(0);
});

// ── accessibility ──────────────────────────────────────────────────────────

test.each([
	['the list', {}],
	['empty', { posts: [], total: 0 }],
	['filtered-empty', { posts: [], filtered: true }],
	['error', { posts: [], failed: true }]
])('%s has no accessibility violations', async (_name, over) => {
	render(PostsPage, { ...base, ...over });

	await expectNoA11yViolations(document.body, UNSTYLED_GEOMETRY);
});
