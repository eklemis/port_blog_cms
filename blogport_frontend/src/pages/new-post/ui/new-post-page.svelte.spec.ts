import { expect, test } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { expectNoA11yViolations } from '$lib/shared/test/a11y';
import NewPostPage from './new-post-page.svelte';

/**
 * `/studio/posts/new` — one column, one form.
 *
 * The page's own job is small: name the screen, say what creating actually
 * does, and hand the new post's id to whatever navigates.
 */

const UNSTYLED_GEOMETRY = { rules: { 'target-size': { enabled: false } } };

test('names the screen', async () => {
	const screen = render(NewPostPage, { oncreated: () => {} });

	await expect.element(screen.getByRole('heading', { level: 1 })).toHaveTextContent('New post');
});

test('says what creating does, before anyone presses the button', async () => {
	// A "New post" screen that says nothing invites the fear that it publishes.
	const screen = render(NewPostPage, { oncreated: () => {} });

	await expect
		.element(screen.getByText('It starts as a draft. Nothing is public until you publish it.'))
		.toBeInTheDocument();
});

test('carries the form that does the work', async () => {
	const screen = render(NewPostPage, { oncreated: () => {} });

	await expect.element(screen.getByRole('textbox', { name: 'Title' })).toBeInTheDocument();
	await expect.element(screen.getByRole('button', { name: 'Create draft' })).toBeInTheDocument();
});

test('has no accessibility violations', async () => {
	render(NewPostPage, { oncreated: () => {} });

	await expectNoA11yViolations(document.body, UNSTYLED_GEOMETRY);
});
