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
	published_at: null
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

	await screen.getByRole('textbox', { name: 'Web address' }).fill('taken');
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
		.element(screen.getByRole('textbox', { name: 'Web address' }))
		.toHaveValue('a-completely-new-title');
});

test('an address someone wrote themselves is left alone', async () => {
	const screen = render(PostEditor, props());

	await screen.getByRole('textbox', { name: 'Web address' }).fill('my-own-address');
	await screen.getByRole('textbox', { name: 'Title' }).fill('A completely new title');

	await expect
		.element(screen.getByRole('textbox', { name: 'Web address' }))
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
		.element(screen.getByRole('textbox', { name: 'Web address' }))
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

test('has no accessibility violations', async () => {
	render(PostEditor, props());

	await expectNoA11yViolations(document.body, UNSTYLED_GEOMETRY);
});
