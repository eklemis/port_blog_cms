import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { expectNoA11yViolations } from '$lib/shared/test/a11y';
import PostEditor from './post-editor.svelte';

/**
 * The editor — J4 steps two and three.
 *
 * Write, and it saves itself. What is tested here is what someone can see: the
 * status line telling the truth, only what moved being sent, and a refused
 * address landing under the address rather than losing the body.
 */

const UNSTYLED_GEOMETRY = { rules: { 'target-size': { enabled: false } } };

const DRAFT = {
	id: 'post-1',
	title: 'Building a CMS',
	slug: 'building-a-cms',
	content: 'The first line.',
	published_at: null,
	topics: [
		{ id: 'topic-1', title: 'Rust' },
		{ id: 'topic-2', title: 'Systems' }
	]
};

function backend(answer: () => Response) {
	return vi.fn<typeof fetch>(async () => answer());
}

const ok = () =>
	new Response(JSON.stringify({ id: 'post-1' }), {
		status: 200,
		headers: { 'content-type': 'application/json' }
	});

const props = (over: Record<string, unknown> = {}) => ({
	post: DRAFT,
	username: 'janedoe',
	fetchFn: backend(ok),
	...over
});

beforeEach(() => vi.useFakeTimers({ shouldAdvanceTime: true }));
afterEach(() => vi.useRealTimers());

test('opens on what was written', async () => {
	const screen = render(PostEditor, props());

	await expect
		.element(screen.getByRole('textbox', { name: 'Title' }))
		.toHaveValue('Building a CMS');
	await expect
		.element(screen.getByRole('textbox', { name: 'Post' }))
		.toHaveValue('The first line.');
});

test('a settled editor says so, and says nothing else', async () => {
	const screen = render(PostEditor, props());

	await expect
		.element(screen.getByRole('status', { name: 'Save state' }))
		.toHaveTextContent('Saved');
});

test('typing says Unsaved changes before it says anything about saving', async () => {
	const screen = render(PostEditor, props());

	await screen.getByRole('textbox', { name: 'Title' }).fill('A new title');

	await expect
		.element(screen.getByRole('status', { name: 'Save state' }))
		.toHaveTextContent('Unsaved changes');
});

test('after a pause it saves, and sends only what moved', async () => {
	const fetchFn = backend(ok);
	const screen = render(PostEditor, props({ fetchFn }));

	await screen.getByRole('textbox', { name: 'Post' }).fill('A longer first line.');
	await vi.advanceTimersByTimeAsync(2000);

	expect(fetchFn).toHaveBeenCalledTimes(1);
	expect(JSON.parse(fetchFn.mock.calls[0][1]?.body as string)).toEqual({
		content: 'A longer first line.'
	});
});

test('retitling a draft moves its address too, and both are sent', async () => {
	// One edit, two changed fields — the address follows the title while the
	// post is a draft, and a PATCH that carried only one of them would leave
	// the post at an address that no longer matches what it is called.
	const fetchFn = backend(ok);
	const screen = render(PostEditor, props({ fetchFn }));

	await screen.getByRole('textbox', { name: 'Title' }).fill('A new title');
	await vi.advanceTimersByTimeAsync(2000);

	expect(JSON.parse(fetchFn.mock.calls[0][1]?.body as string)).toEqual({
		title: 'A new title',
		slug: 'a-new-title'
	});
});

test('a save that lands names the time it landed', async () => {
	const screen = render(PostEditor, props());

	await screen.getByRole('textbox', { name: 'Title' }).fill('A new title');
	await vi.advanceTimersByTimeAsync(2000);

	await expect
		.element(screen.getByRole('status', { name: 'Save state' }))
		.toHaveTextContent(/^Saved \d/);
});

test('a save that fails says so rather than pretending', async () => {
	const fetchFn = backend(
		() =>
			new Response(JSON.stringify({ error: { code: 'INTERNAL_ERROR' } }), {
				status: 500,
				headers: { 'content-type': 'application/json' }
			})
	);
	const screen = render(PostEditor, props({ fetchFn }));

	await screen.getByRole('textbox', { name: 'Title' }).fill('A new title');
	await vi.advanceTimersByTimeAsync(2000);

	await expect
		.element(screen.getByRole('status', { name: 'Save state' }))
		.toHaveTextContent("Couldn't save — retrying");
});

test('a refused address lands under the address and keeps the body', async () => {
	const fetchFn = backend(
		() =>
			new Response(JSON.stringify({ error: { code: 'SLUG_ALREADY_EXISTS' } }), {
				status: 409,
				headers: { 'content-type': 'application/json' }
			})
	);
	const screen = render(PostEditor, props({ fetchFn }));

	await screen.getByRole('textbox', { name: 'Address' }).fill('taken');
	await vi.advanceTimersByTimeAsync(2000);

	await expect.element(screen.getByText('That address is already in use.')).toBeInTheDocument();
	await expect
		.element(screen.getByRole('textbox', { name: 'Post' }))
		.toHaveValue('The first line.');
});

// ── the address ────────────────────────────────────────────────────────────

test('a draft’s address follows the title while it still looks derived', async () => {
	const screen = render(PostEditor, props());

	await screen.getByRole('textbox', { name: 'Title' }).fill('A completely new title');

	await expect
		.element(screen.getByRole('textbox', { name: 'Address' }))
		.toHaveValue('a-completely-new-title');
});

test('an address someone wrote themselves is left alone', async () => {
	const screen = render(PostEditor, props());

	await screen.getByRole('textbox', { name: 'Address' }).fill('my-own-address');
	await screen.getByRole('textbox', { name: 'Title' }).fill('A completely new title');

	await expect
		.element(screen.getByRole('textbox', { name: 'Address' }))
		.toHaveValue('my-own-address');
});

test('a published post keeps its address behind a disclosure that says why', async () => {
	// Editing it breaks a live URL, so it is not sitting open beside the title.
	const screen = render(
		PostEditor,
		props({ post: { ...DRAFT, published_at: '2026-09-01T09:00:00Z' } })
	);

	await expect.element(screen.getByText('Change address')).toBeInTheDocument();
	await expect.element(screen.getByText(/breaks the link/)).toBeInTheDocument();
});

test('a published post’s address never follows the title', async () => {
	const screen = render(
		PostEditor,
		props({ post: { ...DRAFT, published_at: '2026-09-01T09:00:00Z' } })
	);

	await screen.getByRole('textbox', { name: 'Title' }).fill('A completely new title');
	// Behind the disclosure, which is where a live post's address lives.
	await screen.getByText('Change address').click();

	await expect
		.element(screen.getByRole('textbox', { name: 'Address' }))
		.toHaveValue('building-a-cms');
});

// ── publishing ─────────────────────────────────────────────────────────────

test('publishing stamps the time here, and says where it went', async () => {
	// J4 step six: the success toast carries the public link, because the thing
	// that happened is somewhere the person cannot see.
	const fetchFn = backend(ok);
	const screen = render(PostEditor, props({ fetchFn, username: 'janedoe' }));

	await screen.getByRole('button', { name: 'Publish' }).click();

	const sent = JSON.parse(fetchFn.mock.calls[0][1]?.body as string);
	expect(typeof sent.published_at).toBe('string');

	await expect
		.element(screen.getByRole('link', { name: 'View post' }))
		.toHaveAttribute('href', '/janedoe/blog/building-a-cms');
});

test('a post that is live says so in the header, with a way to see it', async () => {
	const screen = render(
		PostEditor,
		props({ post: { ...DRAFT, published_at: '2026-09-01T09:00:00Z' }, username: 'janedoe' })
	);

	// Exact: the disclosure's own sentence says "This post is live", and a
	// substring match would pass on the prose rather than on the pill.
	await expect.element(screen.getByText('Live', { exact: true })).toBeInTheDocument();
	await expect
		.element(screen.getByRole('link', { name: 'View post' }))
		.toHaveAttribute('href', '/janedoe/blog/building-a-cms');
});

test('unpublishing clears the date rather than sending an empty one', async () => {
	// `null` is what puts a post back to draft. An empty string is a date the
	// API would refuse, and omitting the key changes nothing at all.
	const fetchFn = backend(ok);
	const screen = render(
		PostEditor,
		props({
			fetchFn,
			post: { ...DRAFT, published_at: '2026-09-01T09:00:00Z' },
			username: 'janedoe'
		})
	);

	await screen.getByRole('button', { name: 'Unpublish' }).click();

	expect(JSON.parse(fetchFn.mock.calls[0][1]?.body as string)).toEqual({ published_at: null });
});

test('a publish that fails leaves the post a draft and says so', async () => {
	const fetchFn = backend(
		() =>
			new Response(JSON.stringify({ error: { code: 'INTERNAL_ERROR' } }), {
				status: 500,
				headers: { 'content-type': 'application/json' }
			})
	);
	const screen = render(PostEditor, props({ fetchFn, username: 'janedoe' }));

	await screen.getByRole('button', { name: 'Publish' }).click();

	await expect.element(screen.getByText('Something went wrong on our side.')).toBeInTheDocument();
	await expect.element(screen.getByRole('button', { name: 'Publish' })).toBeInTheDocument();
});

// ── Screen / Post editor 32:934 ────────────────────────────────────────────

test('the top bar says what state the post is in, beside the save state', async () => {
	const screen = render(PostEditor, props());

	const bar = screen.getByRole('region', { name: 'Post status' });
	await expect.element(bar.getByText('Draft', { exact: true })).toBeInTheDocument();
	await expect.element(bar.getByRole('status', { name: 'Save state' })).toBeInTheDocument();
	await expect
		.element(bar.getByRole('button', { name: 'Publish', exact: true }))
		.toBeInTheDocument();
});

test('a scheduled post is not called live, because it is not', async () => {
	const future = new Date(Date.now() + 3 * 24 * 3_600_000).toISOString();
	const screen = render(PostEditor, props({ post: { ...DRAFT, published_at: future } }));

	await expect
		.element(
			screen.getByRole('region', { name: 'Post status' }).getByText('Scheduled', { exact: true })
		)
		.toBeInTheDocument();
});

test('a live post says what unpublishing costs before it is pressed', async () => {
	const screen = render(
		PostEditor,
		props({ post: { ...DRAFT, published_at: '2026-09-01T09:00:00Z' } })
	);

	await expect.element(screen.getByText(/stops working/)).toBeInTheDocument();
});

test('the address is shown as it will be read, under the title', async () => {
	// "/janedoe/blog/ building-a-cms-in-rust" in the frame: the fixed part
	// muted, the part that can change in accent ink.
	const screen = render(PostEditor, props());

	await expect.element(screen.getByText('/janedoe/blog/')).toBeInTheDocument();
	await expect
		.element(screen.getByRole('textbox', { name: 'Address' }))
		.toHaveValue('building-a-cms');
});

test('the rail lists the post’s topics', async () => {
	const screen = render(PostEditor, props());

	const topics = screen.getByRole('region', { name: 'Topics' });
	await expect.element(topics.getByText('Rust', { exact: true })).toBeInTheDocument();
	await expect.element(topics.getByText('Systems', { exact: true })).toBeInTheDocument();
});

test('the top bar hides Archive behind a ⋯ menu, at every width', async () => {
	// §06: "a ⋯ menu at the end of the editor's top bar at every width". The
	// call itself is the page's — one feature may not reach into another — so
	// what the editor owes is a saved post and the word that it was asked for.
	const fetchFn = backend(ok);
	const asked: boolean[] = [];
	const screen = render(PostEditor, props({ fetchFn, onarchive: () => asked.push(true) }));

	await screen.getByRole('textbox', { name: 'Title' }).fill('Saved before it goes');
	await screen.getByRole('button', { name: 'More for this post' }).click();
	await screen.getByRole('menuitem', { name: 'Archive' }).click();

	// Flushed first: whatever is on screen is what comes back out of the archive.
	expect(JSON.parse(fetchFn.mock.calls[0][1]?.body as string)).toMatchObject({
		title: 'Saved before it goes'
	});
	await vi.waitFor(() => expect(asked).toEqual([true]));
});

test('Preview saves first, then hands over the share link to open', async () => {
	// §04: the preview is the share link, so the author checks the page a
	// reviewer gets rather than a private render of unsaved words.
	const fetchFn = backend(
		() =>
			new Response(JSON.stringify({ token: 'tok-1' }), {
				status: 200,
				headers: { 'content-type': 'application/json' }
			})
	);
	const opened: string[] = [];
	const screen = render(PostEditor, props({ fetchFn, onpreview: (p: string) => opened.push(p) }));

	await screen.getByRole('textbox', { name: 'Title' }).fill('Saved before the link');
	await screen.getByRole('button', { name: 'Preview' }).click();

	expect(JSON.parse(fetchFn.mock.calls[0][1]?.body as string)).toMatchObject({
		title: 'Saved before the link'
	});
	expect(fetchFn.mock.calls[1][0]).toBe('/api/blog/post-1/preview');
	await vi.waitFor(() => expect(opened).toEqual(['/preview/tok-1']));
});

test('the ⋯ menu carries Preview too, for the width the bar has no room at', async () => {
	// The bar's own Preview button is hidden below md. Without a second way in,
	// 390 would be the one width that cannot preview at all.
	const fetchFn = backend(
		() =>
			new Response(JSON.stringify({ token: 'tok-2' }), {
				status: 200,
				headers: { 'content-type': 'application/json' }
			})
	);
	const opened: string[] = [];
	const screen = render(PostEditor, props({ fetchFn, onpreview: (p: string) => opened.push(p) }));

	await screen.getByRole('button', { name: 'More for this post' }).click();
	await screen.getByRole('menuitem', { name: 'Preview' }).click();

	await vi.waitFor(() => expect(opened).toEqual(['/preview/tok-2']));
});

test('has no accessibility violations', async () => {
	render(PostEditor, props());

	await expectNoA11yViolations(document.body, UNSTYLED_GEOMETRY);
});

// ── topics ─────────────────────────────────────────────────────────────────

test('adding a topic sends one request and puts the chip on the post', async () => {
	// §03: each chip is its own request. This one is the whole of it.
	const fetchFn = backend(() => new Response(null, { status: 204 }));
	const screen = render(
		PostEditor,
		props({
			fetchFn,
			availableTopics: [
				{ id: 'topic-1', title: 'Rust' },
				{ id: 'topic-3', title: 'Architecture' }
			]
		})
	);

	await screen.getByRole('button', { name: 'Add a topic' }).click();
	await screen.getByRole('button', { name: 'Architecture' }).click();

	await vi.waitFor(() =>
		expect(screen.getByRole('button', { name: 'Remove Architecture' }).elements()).toHaveLength(1)
	);
	expect(fetchFn.mock.calls[0][0]).toBe('/api/blog/post-1/topics');
	expect(JSON.parse(String(fetchFn.mock.calls[0][1]?.body))).toEqual({ topic_id: 'topic-3' });
});

test('removing a topic takes the chip off, and says so to the server', async () => {
	const fetchFn = backend(() => new Response(null, { status: 204 }));
	const screen = render(PostEditor, props({ fetchFn }));

	await screen.getByRole('button', { name: 'Remove Systems' }).click();

	await vi.waitFor(() =>
		expect(screen.getByRole('button', { name: 'Remove Systems' }).elements()).toHaveLength(0)
	);
	expect(fetchFn.mock.calls[0][1]?.method).toBe('DELETE');
});

test('a refused chip leaves the post as it was, and says what happened', async () => {
	// One failure does not roll back the others, and it does not pretend either:
	// the chip that did not attach is not drawn as though it had.
	const fetchFn = backend(
		() =>
			new Response(JSON.stringify({ error: { code: 'INTERNAL_ERROR' } }), {
				status: 500,
				headers: { 'content-type': 'application/json' }
			})
	);
	const screen = render(PostEditor, props({ fetchFn }));

	await screen.getByRole('button', { name: 'Remove Systems' }).click();

	// `role="status"`, never `role="alert"`: assertive is reserved, and a chip
	// that did not attach is not an emergency.
	await expect
		.element(screen.getByRole('status').filter({ hasText: 'Something went wrong' }))
		.toBeInTheDocument();
	await expect.element(screen.getByRole('button', { name: 'Remove Systems' })).toBeInTheDocument();
});

test('a topic that does not exist yet is created, then attached', async () => {
	// §03: "Create & attach". Two requests, in that order, because the second
	// needs the id the first hands back.
	const calls: string[] = [];
	const fetchFn = vi.fn<typeof fetch>(async (url) => {
		calls.push(String(url));
		if (String(url) === '/api/topics') {
			return new Response(JSON.stringify({ data: { id: 'topic-9', title: 'Postgres' } }), {
				status: 201,
				headers: { 'content-type': 'application/json' }
			});
		}
		return new Response(null, { status: 204 });
	});
	const screen = render(PostEditor, props({ fetchFn, availableTopics: [] }));

	await screen.getByRole('button', { name: 'Add a topic' }).click();
	await screen.getByRole('textbox', { name: 'Search topics' }).fill('Postgres');
	await screen.getByRole('button', { name: 'Create “Postgres”' }).click();

	await vi.waitFor(() =>
		expect(screen.getByRole('button', { name: 'Remove Postgres' }).elements()).toHaveLength(1)
	);
	expect(calls).toEqual(['/api/topics', '/api/blog/post-1/topics']);
});
