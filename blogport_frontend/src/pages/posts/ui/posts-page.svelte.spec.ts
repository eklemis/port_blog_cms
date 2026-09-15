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
		updated_at: '2026-09-06T12:00:00Z',
		topics: [
			{ id: 'topic-1', title: 'Rust', description: '' },
			{ id: 'topic-2', title: 'Distributed Systems', description: '' }
		]
	},
	{
		id: '2',
		title: 'Why hexagonal, actually',
		published_at: null,
		updated_at: '2026-09-08T07:00:00Z',
		topics: []
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

test('a row opens its post, because a list of titles you cannot click is a report', async () => {
	const screen = render(PostsPage, base);

	// Scoped to the table: the phone's cards carry the same link, and both are
	// in the DOM with one hidden by CSS.
	await expect
		.element(screen.getByRole('table').getByRole('link', { name: 'Building a CMS in Rust' }))
		.toHaveAttribute('href', '/studio/posts/1');
});

test('the archive is one step away, because nothing else leads there', async () => {
	const screen = render(PostsPage, base);

	await expect
		.element(screen.getByRole('link', { name: 'Archived' }))
		.toHaveAttribute('href', '/studio/posts/archive');
});

test('each row names its topics, straight from the list', async () => {
	// Screen / Posts list 11:2: Title · Status · Topics · Updated, the topics
	// as one line joined with a middle dot. The card carries them now, loaded
	// once for the page — a column, not one request per row.
	const screen = render(PostsPage, base);

	const headers = screen
		.getByRole('columnheader')
		.elements()
		.map((el) => el.textContent?.trim());
	expect(headers).toEqual(['Title', 'Status', 'Topics', 'Updated']);

	const first = screen.getByRole('row').nth(1);
	await expect.element(first.getByText('Rust · Distributed Systems')).toBeInTheDocument();
});

test('a post with no topics shows the dash the frame draws', async () => {
	// The frame's "Consensus reading list" row. Before topics were on the card,
	// a dash in every row would have been a claim nobody could back; now empty
	// means the post has none, which is exactly what a dash says.
	const screen = render(PostsPage, base);

	const second = screen.getByRole('row').nth(2);
	await expect.element(second.getByText('—', { exact: true })).toBeInTheDocument();
});

test('lists the posts with the state each is in', async () => {
	const screen = render(PostsPage, base);

	await expect
		.element(screen.getByRole('table').getByText('Building a CMS in Rust'))
		.toBeInTheDocument();
	// Exact: "Drafts & published" and "Drafts only" are options in the filter,
	// and a substring match would find those too.
	const table = screen.getByRole('table');
	await expect.element(table.getByText('Live', { exact: true })).toBeInTheDocument();
	await expect.element(table.getByText('Draft', { exact: true })).toBeInTheDocument();
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

	await expect.element(screen.getByText('No posts yet', { exact: true })).toBeInTheDocument();
	await expect
		.element(screen.getByText('Your first post is the one that turns this into a blog.'))
		.toBeInTheDocument();
	await expect
		.element(screen.getByRole('link', { name: 'Write your first post' }))
		.toHaveAttribute('href', '/studio/posts/new');
});

test('with no posts at all there is no toolbar, because there is nothing to search', async () => {
	// Screen / Posts — empty: "offering controls that cannot do anything is noise."
	const screen = render(PostsPage, { ...base, posts: [], total: 0 });

	expect(screen.getByRole('searchbox').elements()).toHaveLength(0);
});

test('filtered-empty is a different screen from empty', async () => {
	const screen = render(PostsPage, {
		...base,
		posts: [],
		total: 0,
		everything: 24,
		filtered: true,
		published: 'false',
		topic: 'topic-1',
		topics: TOPICS
	});

	await expect
		.element(screen.getByText('No posts match those filters', { exact: true }))
		.toBeInTheDocument();
	// It says what does exist, and which filter is the reason, so nobody thinks
	// their work is gone. The toolbar stays, because the filters are the answer.
	await expect
		.element(screen.getByText('You have 24 posts — none of them are drafts tagged “Rust”.'))
		.toBeInTheDocument();
	await expect.element(screen.getByRole('searchbox')).toBeInTheDocument();
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

	await expect
		.element(screen.getByRole('heading', { name: 'Couldn’t load your posts' }))
		.toBeInTheDocument();
	await expect
		.element(screen.getByText('Something went wrong on our side. Your posts are safe.'))
		.toBeInTheDocument();
	await expect.element(screen.getByRole('button', { name: 'Try again' })).toBeInTheDocument();
	// The toolbar stays, so the query is not lost on retry.
	await expect.element(screen.getByRole('searchbox')).toBeInTheDocument();
});

test('loading is rows, not a spinner, and it is announced', async () => {
	const screen = render(PostsPage, { ...base, posts: [], loading: true });

	await expect.element(screen.getByRole('status')).toHaveTextContent('Loading posts');
	expect(screen.getByText('No posts yet', { exact: true }).elements()).toHaveLength(0);
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

	await expect.element(screen.getByRole('button', { name: 'Previous page' })).toBeDisabled();
	await expect.element(screen.getByRole('button', { name: 'Next page' })).not.toBeDisabled();
});

test('one page of results needs no pager at all', async () => {
	const screen = render(PostsPage, { ...base, total: 2 });

	expect(screen.getByRole('button', { name: 'Next page' }).elements()).toHaveLength(0);
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

// ── Screen / Posts list 11:2 and Mobile / Posts list 73:2 ──────────────────

test('an active filter becomes a chip that takes it off', async () => {
	// Screen / Posts — filtered empty 84:557: "Rust ×", "Drafts ×". The filters
	// are the reason for what is on screen, so they stay visible as themselves.
	const asked: Record<string, string | null>[] = [];
	const screen = render(PostsPage, {
		...base,
		topics: TOPICS,
		topic: 'topic-1',
		published: 'false',
		onquery: (c: Record<string, string | null>) => asked.push(c)
	});

	await screen.getByRole('button', { name: 'Rust, remove filter' }).first().click();
	await screen.getByRole('button', { name: 'Drafts, remove filter' }).first().click();

	expect(asked).toEqual([
		{ topic_id: null, page: null },
		{ published: null, page: null }
	]);
});

test('on a phone the status filter is three chips, and one of them is on', async () => {
	// Mobile / Posts list: All · Drafts · Published.
	const asked: Record<string, string | null>[] = [];
	const screen = render(PostsPage, {
		...base,
		onquery: (c: Record<string, string | null>) => asked.push(c)
	});

	const group = screen.getByRole('group', { name: 'Show' });
	await expect
		.element(group.getByRole('button', { name: 'All' }))
		.toHaveAttribute('aria-pressed', 'true');
	await group.getByRole('button', { name: 'Drafts' }).click();

	expect(asked).toEqual([{ published: 'false', page: null }]);
});

test('on a phone each post is a card with its status, topics and age', async () => {
	const screen = render(PostsPage, base);

	const cards = screen.getByRole('listitem');
	await expect
		.element(cards.nth(0).getByRole('link', { name: 'Building a CMS in Rust' }))
		.toBeInTheDocument();
	await expect
		.element(cards.nth(0).getByText(/^Rust · Distributed Systems · /))
		.toBeInTheDocument();
});

test('a scheduled post’s card says when it goes live', async () => {
	const future = new Date(Date.now() + 3 * 24 * 3_600_000).toISOString();
	const screen = render(PostsPage, {
		...base,
		posts: [{ ...base.posts[0], published_at: future }]
	});

	await expect
		.element(
			screen
				.getByRole('listitem')
				.first()
				.getByText(/goes live in 3 days/)
		)
		.toBeInTheDocument();
});

test('the pager numbers its pages', async () => {
	const screen = render(PostsPage, { ...base, page: 2, total: 24 });

	await expect
		.element(screen.getByRole('button', { name: 'Page 2' }))
		.toHaveAttribute('aria-current', 'page');
});
