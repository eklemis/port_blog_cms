import { expect, test, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { expectNoA11yViolations } from '$lib/shared/test/a11y';
import PostsArchivePage from './posts-archive-page.svelte';

/**
 * The archive: where the second and third rungs of destruction live.
 *
 * Restore is one click and no confirm. Delete forever is a dialog naming the
 * post and asking for its title. Both wait for the server.
 */

const UNSTYLED_GEOMETRY = { rules: { 'target-size': { enabled: false } } };

const posts = [
	{ id: 'post-1', title: 'Building a CMS', updated_at: '2026-09-06T12:00:00Z' },
	{ id: 'post-2', title: 'Why hexagonal', updated_at: '2026-09-01T12:00:00Z' }
];

const respond = (status: number) => vi.fn<typeof fetch>(async () => new Response(null, { status }));

const props = (over: Record<string, unknown> = {}) => ({
	posts,
	total: 2,
	page: 1,
	perPage: 10,
	onchanged: () => {},
	fetchFn: respond(204),
	...over
});

test('lists what is archived, and says it is', async () => {
	const screen = render(PostsArchivePage, props());

	await expect.element(screen.getByText('Building a CMS')).toBeInTheDocument();
	expect(screen.getByText('Archived', { exact: true }).elements()).toHaveLength(2);
});

test('restore is one click, no confirm, and the list is told', async () => {
	const changed: boolean[] = [];
	const fetchFn = respond(204);
	const screen = render(PostsArchivePage, props({ fetchFn, onchanged: () => changed.push(true) }));

	await screen.getByRole('button', { name: 'Restore Building a CMS' }).click();

	expect(screen.getByRole('dialog').elements()).toHaveLength(0);
	expect(fetchFn.mock.calls[0][0]).toBe('/api/blog/post-1/restore');
	await vi.waitFor(() => expect(changed).toEqual([true]));
});

test('restoring says where the post went, because it went somewhere unseen', async () => {
	const screen = render(PostsArchivePage, props());

	await screen.getByRole('button', { name: 'Restore Building a CMS' }).click();

	await expect.element(screen.getByText('Restored. It is back in your posts.')).toBeInTheDocument();
});

test('delete forever asks for the title before it does anything', async () => {
	const fetchFn = respond(204);
	const screen = render(PostsArchivePage, props({ fetchFn }));

	await screen.getByRole('button', { name: 'Delete Building a CMS forever' }).click();

	await expect.element(screen.getByRole('dialog')).toBeInTheDocument();
	expect(fetchFn).not.toHaveBeenCalled();
});

test('typing the title and confirming deletes that post and no other', async () => {
	const changed: boolean[] = [];
	const fetchFn = respond(204);
	const screen = render(PostsArchivePage, props({ fetchFn, onchanged: () => changed.push(true) }));

	await screen.getByRole('button', { name: 'Delete Building a CMS forever' }).click();
	await screen.getByRole('textbox').fill('Building a CMS');
	await screen.getByRole('button', { name: 'Delete forever' }).click();

	expect(fetchFn.mock.calls[0][0]).toBe('/api/blog/post-1/hard');
	await vi.waitFor(() => expect(changed).toEqual([true]));
	await vi.waitFor(() => expect(screen.getByRole('dialog').elements()).toHaveLength(0));
});

test('a purge that fails keeps the dialog open and says so', async () => {
	// Closing it would imply it worked. The post is still here, and so is the
	// person's decision.
	const screen = render(PostsArchivePage, props({ fetchFn: respond(500) }));

	await screen.getByRole('button', { name: 'Delete Building a CMS forever' }).click();
	await screen.getByRole('textbox').fill('Building a CMS');
	await screen.getByRole('button', { name: 'Delete forever' }).click();

	// Inside the dialog. Everything behind a modal dialog is inert, so a message
	// rendered on the page would be visible and unannounced at once.
	await expect
		.element(screen.getByRole('dialog').getByText('Something went wrong on our side.'))
		.toBeInTheDocument();
});

test('an empty archive says what it is for', async () => {
	const screen = render(PostsArchivePage, props({ posts: [], total: 0 }));

	await expect.element(screen.getByText('Nothing archived.')).toBeInTheDocument();
});

test('has no accessibility violations', async () => {
	render(PostsArchivePage, props());

	await expectNoA11yViolations(document.body, UNSTYLED_GEOMETRY);
});
