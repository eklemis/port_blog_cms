import { expect, test } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { expectNoA11yViolations } from '$lib/shared/test/a11y';
import ProjectEditorPage from './project-editor-page.svelte';

/**
 * `/studio/projects/[id]` — two shapes.
 *
 * A project that is not yours comes back as not found rather than forbidden,
 * so the refusal and the missing project are the same screen. Neither bounces
 * anyone through login.
 */

/** These specs render without the stylesheet, so hit areas are not real. */
const UNSTYLED_GEOMETRY = { rules: { 'target-size': { enabled: false } } };

const PROJECT = {
	id: 'p-1',
	title: 'Blogport CMS',
	slug: 'blogport-cms',
	description: 'A portfolio CMS.',
	tech_stack: ['Rust'],
	topics: [],
	repo_url: null,
	live_demo_url: null
};

const props = (over: Record<string, unknown> = {}) => ({
	project: PROJECT,
	username: 'janedoe',
	screenshots: [],
	availableTopics: [],
	...over
});

test('shows the editor for a project that is yours', async () => {
	const screen = render(ProjectEditorPage, props());

	await expect.element(screen.getByRole('textbox', { name: 'Title' })).toBeInTheDocument();
});

test('a project that is not yours is refused in plain words, with a way back', async () => {
	const screen = render(ProjectEditorPage, props({ project: null, denied: true }));

	await expect.element(screen.getByText('We couldn’t find that project.')).toBeInTheDocument();
	await expect
		.element(screen.getByRole('link', { name: 'Back to projects' }))
		.toHaveAttribute('href', '/studio/projects');
});

test('the refusal never offers a way to sign in again', async () => {
	// The person is signed in. Someone else's project is not an auth problem,
	// and a login prompt would say it was.
	const screen = render(ProjectEditorPage, props({ project: null, denied: true }));

	expect(screen.container.innerHTML).not.toMatch(/sign in|log in/i);
});

test('has no accessibility violations', async () => {
	render(ProjectEditorPage, props());

	await expectNoA11yViolations(document.body, UNSTYLED_GEOMETRY);
});
