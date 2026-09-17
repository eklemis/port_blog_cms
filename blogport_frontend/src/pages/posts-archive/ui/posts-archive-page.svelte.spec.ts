import { expect, test, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { expectNoA11yViolations } from '$lib/shared/test/a11y';
import PostsArchivePage from './posts-archive-page.svelte';

/**
 * The archive — Screen / Posts archive 71:126, Mobile / Posts archive 97:2744.
 *
 * §06's second and third rungs: restore is one click and no confirm, purge is
 * a dialog asking for the title. Rows can be selected and acted on together,
 * and a batch that partly fails leaves the failed rows selected with their own
 * reason.
 */

const UNSTYLED_GEOMETRY = { rules: { 'target-size': { enabled: false } } };

const posts = [
	{
		id: 'post-1',
		title: 'Old benchmarking post',
		deleted_at: '2026-04-10T12:00:00Z',
		updated_at: '2026-04-10T12:00:00Z'
	},
	{
		id: 'post-2',
		title: 'Draft that went nowhere',
		deleted_at: '2026-03-02T12:00:00Z',
		updated_at: '2026-03-02T12:00:00Z'
	},
	{
		id: 'post-3',
		title: 'Notes on the old scheduler',
		deleted_at: '2026-01-20T12:00:00Z',
		updated_at: '2026-01-20T12:00:00Z'
	}
];

type Answer = { status: number; body?: unknown };

/** Answers by path; a bulk call answers with every id succeeding unless told. */
function backend(answers: Record<string, Answer> = {}) {
	return vi.fn<typeof fetch>(async (input, init) => {
		const url = String(input);
		const key = Object.keys(answers).find((candidate) => url.startsWith(candidate));
		if (key) {
			const { status, body } = answers[key];
			return body === undefined
				? new Response(null, { status })
				: new Response(JSON.stringify(body), {
						status,
						headers: { 'content-type': 'application/json' }
					});
		}
		if (url === '/api/blog/bulk') {
			const { ids } = JSON.parse(init?.body as string);
			return new Response(JSON.stringify({ succeeded: ids, failed: [] }), {
				status: 200,
				headers: { 'content-type': 'application/json' }
			});
		}
		return new Response(null, { status: 204 });
	});
}

const props = (over: Record<string, unknown> = {}) => ({
	posts,
	total: 3,
	page: 1,
	perPage: 10,
	onchanged: () => {},
	fetchFn: backend(),
	...over
});

const sent = (fetchFn: ReturnType<typeof backend>, n = 0) =>
	JSON.parse(fetchFn.mock.calls[n][1]?.body as string);

// ── the screen ─────────────────────────────────────────────────────────────

test('names itself as the archive, once, rather than labelling every row', async () => {
	const screen = render(PostsArchivePage, props());

	await expect.element(screen.getByRole('heading', { level: 1 })).toHaveTextContent('Archive');
	// The pill sits beside the title. A row saying "Archived" on a screen that
	// only lists archived posts is noise, and the frame does not draw one.
	const row = screen.getByRole('row').nth(1);
	expect(row.getByText('Archived', { exact: true }).elements()).toHaveLength(0);
});

test('says what the archive is, in the frame’s words', async () => {
	const screen = render(PostsArchivePage, props());

	await expect
		.element(
			screen.getByText(
				'Archived posts are not public and not in your main list. Restore puts one back in whatever state it was in; purge is permanent and asks you to type the title.'
			)
		)
		.toBeInTheDocument();
	await expect
		.element(screen.getByRole('link', { name: 'Back to posts' }))
		.toHaveAttribute('href', '/studio/posts');
});

test('each row says when it was archived, as a month', async () => {
	const screen = render(PostsArchivePage, props());

	await expect.element(screen.getByRole('columnheader', { name: 'Archived' })).toBeInTheDocument();
	await expect.element(screen.getByText('Apr 2026').first()).toBeInTheDocument();
});

test('the date is when it was archived, not when it was last touched', async () => {
	// It used to read `updated_at`, which was only ever true by accident — every
	// write path happened to keep `is_deleted` set, and a trigger stamped the
	// row. The backend now records the archiving itself, on the `is_deleted`
	// transition, so an edit after archiving no longer moves the date.
	const archived = [
		{
			...posts[0],
			deleted_at: '2026-04-10T12:00:00Z',
			updated_at: '2026-09-01T12:00:00Z'
		}
	];
	const screen = render(PostsArchivePage, props({ posts: archived, total: 1 }));

	await expect.element(screen.getByText('Apr 2026').first()).toBeInTheDocument();
	expect(screen.container.textContent).not.toContain('Sep 2026');
});

test('a row the server sent no archive date for says so rather than guessing', async () => {
	const screen = render(
		PostsArchivePage,
		props({ posts: [{ ...posts[0], deleted_at: null }], total: 1 })
	);

	await expect.element(screen.getByText('—').first()).toBeInTheDocument();
});

// ── one at a time ──────────────────────────────────────────────────────────

test('restore is one click, no confirm, and says where the post went', async () => {
	const changed: boolean[] = [];
	const fetchFn = backend();
	const screen = render(PostsArchivePage, props({ fetchFn, onchanged: () => changed.push(true) }));

	await screen.getByRole('button', { name: 'Restore Old benchmarking post' }).first().click();

	expect(fetchFn.mock.calls[0][0]).toBe('/api/blog/post-1/restore');
	expect(screen.getByRole('dialog').elements()).toHaveLength(0);
	await expect.element(screen.getByText('Restored. It is back in your posts.')).toBeInTheDocument();
	expect(changed).toEqual([true]);
});

test('purge asks for the title before it does anything', async () => {
	const fetchFn = backend();
	const screen = render(PostsArchivePage, props({ fetchFn }));

	await screen.getByRole('button', { name: 'Purge Old benchmarking post' }).first().click();

	await expect.element(screen.getByRole('dialog')).toBeInTheDocument();
	expect(fetchFn).not.toHaveBeenCalled();

	await screen.getByRole('textbox').fill('Old benchmarking post');
	await screen.getByRole('button', { name: 'Purge', exact: true }).click();

	expect(fetchFn.mock.calls[0][0]).toBe('/api/blog/post-1/hard');
});

test('a purge that fails says so inside the dialog, which stays open', async () => {
	const screen = render(
		PostsArchivePage,
		props({ fetchFn: backend({ '/api/blog/post-1/hard': { status: 500, body: {} } }) })
	);

	await screen.getByRole('button', { name: 'Purge Old benchmarking post' }).first().click();
	await screen.getByRole('textbox').fill('Old benchmarking post');
	await screen.getByRole('button', { name: 'Purge', exact: true }).click();

	await expect
		.element(screen.getByRole('dialog').getByText('Something went wrong on our side.'))
		.toBeInTheDocument();
});

// ── together ───────────────────────────────────────────────────────────────

test('selecting rows brings up the bar that acts on them', async () => {
	const screen = render(PostsArchivePage, props());

	expect(screen.getByText('2 selected').elements()).toHaveLength(0);

	await screen.getByRole('checkbox', { name: 'Select Old benchmarking post' }).first().click();
	await screen.getByRole('checkbox', { name: 'Select Draft that went nowhere' }).first().click();

	await expect.element(screen.getByText('2 selected').first()).toBeInTheDocument();
	await expect.element(screen.getByRole('button', { name: 'Restore both' })).toBeInTheDocument();
	await expect.element(screen.getByRole('button', { name: 'Purge both' })).toBeInTheDocument();
});

test('restoring a selection is one call, not one per row', async () => {
	const fetchFn = backend();
	const screen = render(PostsArchivePage, props({ fetchFn }));

	await screen.getByRole('checkbox', { name: 'Select Old benchmarking post' }).first().click();
	await screen.getByRole('checkbox', { name: 'Select Draft that went nowhere' }).first().click();
	await screen.getByRole('button', { name: 'Restore both' }).click();

	expect(fetchFn).toHaveBeenCalledTimes(1);
	expect(sent(fetchFn)).toEqual({ op: 'restore', ids: ['post-1', 'post-2'] });
});

test('a batch that partly fails keeps the failed row selected, with its reason', async () => {
	// The frame's note: "success: true means the batch ran, not that every item
	// did. Failed rows stay selected with their own reason."
	const fetchFn = backend({
		'/api/blog/bulk': {
			status: 200,
			body: {
				succeeded: ['post-1'],
				failed: [{ id: 'post-2', code: 'POST_NOT_FOUND', message: 'x' }]
			}
		}
	});
	const screen = render(PostsArchivePage, props({ fetchFn }));

	await screen.getByRole('checkbox', { name: 'Select Old benchmarking post' }).first().click();
	await screen.getByRole('checkbox', { name: 'Select Draft that went nowhere' }).first().click();
	await screen.getByRole('button', { name: 'Restore both' }).click();

	await expect.element(screen.getByText('1 selected').first()).toBeInTheDocument();
	await expect
		.element(screen.getByRole('checkbox', { name: 'Select Draft that went nowhere' }).first())
		.toBeChecked();
	await expect
		.element(screen.getByText('That post is no longer in the archive.').first())
		.toBeInTheDocument();
});

test('purging a selection asks for how many, then sends one call', async () => {
	// A selection has no title, so the number is typed instead — §06.
	const fetchFn = backend();
	const screen = render(PostsArchivePage, props({ fetchFn }));

	await screen.getByRole('checkbox', { name: 'Select Old benchmarking post' }).first().click();
	await screen.getByRole('checkbox', { name: 'Select Draft that went nowhere' }).first().click();
	await screen.getByRole('button', { name: 'Purge both' }).click();

	expect(fetchFn).not.toHaveBeenCalled();
	// "Type 2 to purge 2 posts." — the number, which is the thing someone gets
	// wrong by ticking one row too many.
	await expect.element(screen.getByText('Type 2 to purge 2 posts.')).toBeInTheDocument();
	await screen.getByRole('textbox').fill('2');
	await screen.getByRole('button', { name: 'Purge both', exact: true }).last().click();

	expect(sent(fetchFn)).toEqual({ op: 'hard_delete', ids: ['post-1', 'post-2'] });
});

// ── states ─────────────────────────────────────────────────────────────────

test('an empty archive says what it is for', async () => {
	const screen = render(PostsArchivePage, props({ posts: [], total: 0 }));

	await expect.element(screen.getByText('Nothing archived', { exact: true })).toBeInTheDocument();
});

test('has no accessibility violations', async () => {
	render(PostsArchivePage, props());

	await expectNoA11yViolations(document.body, UNSTYLED_GEOMETRY);
});
