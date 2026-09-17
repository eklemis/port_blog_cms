import { expect, test, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { expectNoA11yViolations } from '$lib/shared/test/a11y';
import PostEditorPage from './post-editor-page.svelte';

/**
 * `/studio/posts/[id]`.
 *
 * Two shapes: the editor, and the page someone reaches by pasting a URL for a
 * post that is not theirs. J4 asks for a plain sentence and a route back to the
 * list, never a bounce through login.
 */

const UNSTYLED_GEOMETRY = { rules: { 'target-size': { enabled: false } } };

const post = {
	id: 'post-1',
	title: 'Building a CMS',
	slug: 'building-a-cms',
	content: 'The first line.',
	published_at: null
};

test('shows the editor for a post that is yours', async () => {
	const screen = render(PostEditorPage, { post, username: 'janedoe', denied: false });

	await expect.element(screen.getByRole('textbox', { name: 'Title' })).toBeInTheDocument();
});

test('someone else’s post is a plain sentence with a way back', async () => {
	// Only reachable by pasting a URL, and never a bounce through login.
	const screen = render(PostEditorPage, { post: null, username: 'janedoe', denied: true });

	await expect.element(screen.getByText("You don't have access to this post.")).toBeInTheDocument();
	await expect.element(screen.getByRole('link', { name: 'Back to posts' })).toBeInTheDocument();
	expect(screen.getByRole('textbox', { name: 'Title' }).elements()).toHaveLength(0);
});

test('archiving from the editor calls it in, then leaves for the list', async () => {
	const fetchFn = vi.fn<typeof fetch>(async () => new Response(null, { status: 204 }));
	const left: boolean[] = [];
	const screen = render(PostEditorPage, {
		post,
		username: 'janedoe',
		denied: false,
		fetchFn,
		onarchived: () => left.push(true)
	});

	await screen.getByRole('button', { name: 'More for this post' }).click();
	await screen.getByRole('menuitem', { name: 'Archive' }).click();

	await vi.waitFor(() => expect(fetchFn.mock.calls[0][0]).toBe('/api/blog/post-1'));
	expect(fetchFn.mock.calls[0][1]?.method).toBe('DELETE');
	await vi.waitFor(() => expect(left).toEqual([true]));
});

test('an archive that fails says so and stays put', async () => {
	const fetchFn = vi.fn<typeof fetch>(
		async () =>
			new Response(JSON.stringify({ error: { code: 'INTERNAL_ERROR' } }), {
				status: 500,
				headers: { 'content-type': 'application/json' }
			})
	);
	const left: boolean[] = [];
	const screen = render(PostEditorPage, {
		post,
		username: 'janedoe',
		denied: false,
		fetchFn,
		onarchived: () => left.push(true)
	});

	await screen.getByRole('button', { name: 'More for this post' }).click();
	await screen.getByRole('menuitem', { name: 'Archive' }).click();

	await expect.element(screen.getByText('Something went wrong on our side.')).toBeInTheDocument();
	expect(left).toEqual([]);
});

test('has no accessibility violations', async () => {
	render(PostEditorPage, { post, username: 'janedoe', denied: false });

	await expectNoA11yViolations(document.body, UNSTYLED_GEOMETRY);
});

test('nor does the no-access page', async () => {
	render(PostEditorPage, { post: null, username: 'janedoe', denied: true });

	await expectNoA11yViolations(document.body, UNSTYLED_GEOMETRY);
});
