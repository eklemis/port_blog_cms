import { expect, test } from 'vitest';
import { createRawSnippet } from 'svelte';
import { render } from 'vitest-browser-svelte';
import { expectNoA11yViolations } from '$lib/shared/test/a11y';
import EmptyState from './empty-state.svelte';

/**
 * The panel a list shows when it has no rows to show.
 *
 * Three usages, not three components: empty, filtered-empty and error. They are
 * one shape on purpose — what changes between them is the sentence and the way
 * out, and conflating *those* is the mistake worth guarding against, not the
 * box they sit in.
 */

const UNSTYLED_GEOMETRY = { rules: { 'target-size': { enabled: false } } };

const action = createRawSnippet(() => ({ render: () => '<button>Write your first post</button>' }));

test('says what has happened and what to do about it', async () => {
	const screen = render(EmptyState, {
		title: 'No posts yet.',
		message: 'This is where everything you write will live.',
		action
	});

	await expect.element(screen.getByText('No posts yet.')).toBeInTheDocument();
	await expect
		.element(screen.getByText('This is where everything you write will live.'))
		.toBeInTheDocument();
	await expect
		.element(screen.getByRole('button', { name: 'Write your first post' }))
		.toBeInTheDocument();
});

test('the title is a heading, so it is reachable by one', async () => {
	// A list that has gone empty is where someone lands; skipping by headings
	// should find the reason rather than nothing at all.
	const screen = render(EmptyState, { title: 'No posts match those filters.', message: 'x' });

	await expect
		.element(screen.getByRole('heading', { name: 'No posts match those filters.' }))
		.toBeInTheDocument();
});

test('an action is optional — an error that cannot be retried offers none', async () => {
	const screen = render(EmptyState, { title: 'Nothing here', message: 'x' });

	expect(screen.getByRole('button').elements()).toHaveLength(0);
});

test('has no accessibility violations', async () => {
	render(EmptyState, { title: 'No posts yet.', message: 'A sentence.', action });

	await expectNoA11yViolations(document.body, UNSTYLED_GEOMETRY);
});
