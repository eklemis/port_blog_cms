import { expect, test } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { expectNoA11yViolations } from '$lib/shared/test/a11y';
import NewProjectPage from './new-project-page.svelte';

/** `/studio/projects/new`. */

/** These specs render without the stylesheet, so hit areas are not real. */
const UNSTYLED_GEOMETRY = { rules: { 'target-size': { enabled: false } } };

test('names the screen and the way back to the list', async () => {
	const screen = render(NewProjectPage, { oncreated: () => {} });

	await expect
		.element(screen.getByRole('heading', { level: 1, name: 'New project' }))
		.toBeInTheDocument();
	await expect
		.element(screen.getByRole('link', { name: 'Projects' }))
		.toHaveAttribute('href', '/studio/projects');
});

test('carries the form, and the warning that it publishes', async () => {
	const screen = render(NewProjectPage, { oncreated: () => {} });

	await expect.element(screen.getByRole('button', { name: 'Add project' })).toBeInTheDocument();
	await expect
		.element(screen.getByText('Projects are visible on your public page as soon as they’re added.'))
		.toBeInTheDocument();
});

test('has no accessibility violations', async () => {
	render(NewProjectPage, { oncreated: () => {} });

	await expectNoA11yViolations(document.body, UNSTYLED_GEOMETRY);
});
