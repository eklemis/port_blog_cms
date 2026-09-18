import { expect, test } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { expectNoA11yViolations } from '$lib/shared/test/a11y';
import PublicProjectsPage from './public-projects-page.svelte';

/** Screen / Public projects 82:579 · Mobile / Public projects 88:2225. */

/** These specs render without the stylesheet, so hit areas are not real. */
const UNSTYLED_GEOMETRY = { rules: { 'target-size': { enabled: false } } };

const props = (over: Record<string, unknown> = {}) => ({
	author: { username: 'janedoe', fullName: 'Jane Doe', avatarSrc: null },
	projects: [
		{
			slug: 'blogport-cms',
			title: 'Blogport CMS',
			description: 'A portfolio CMS across four deployed services.',
			techStack: ['Rust', 'Actix', 'SvelteKit'],
			topics: [{ id: 't-rust', title: 'Rust' }],
			cover: { src: 'https://api.example.test/api/public/media/c1/large', alt: 'A screenshot' },
			repoUrl: 'https://github.com/eklemis/port_blog_cms',
			demoUrl: 'https://example.test/demo'
		},
		{
			slug: 'heap-visualiser',
			title: 'Heap visualiser',
			description: 'A teaching tool for binary heap operations.',
			techStack: ['TypeScript'],
			topics: [],
			cover: null,
			repoUrl: null,
			demoUrl: null
		}
	],
	total: 8,
	page: 1,
	perPage: 10,
	filter: null,
	...over
});

test('lists each project with what it is and what it is made of', async () => {
	const screen = render(PublicProjectsPage, props());

	await expect
		.element(screen.getByRole('link', { name: 'Blogport CMS' }))
		.toHaveAttribute('href', '/janedoe/projects/blogport-cms');
	await expect
		.element(screen.getByText('A portfolio CMS across four deployed services.'))
		.toBeInTheDocument();
	await expect.element(screen.getByText('SvelteKit')).toBeInTheDocument();
});

test('offers the repository and the running thing, when there are any', async () => {
	const screen = render(PublicProjectsPage, props());

	await expect
		.element(screen.getByRole('link', { name: 'repo' }))
		.toHaveAttribute('href', 'https://github.com/eklemis/port_blog_cms');
	await expect.element(screen.getByRole('link', { name: 'demo' })).toBeInTheDocument();
});

test('a project with nowhere to click shows no links at all', async () => {
	// The emptiest card the frame allows: no cover, no topics, one tech label
	// and neither link.
	const screen = render(PublicProjectsPage, props({ projects: [props().projects[1]], total: 1 }));

	expect(screen.getByRole('link', { name: 'repo' }).elements()).toHaveLength(0);
	expect(screen.getByRole('link', { name: 'demo' }).elements()).toHaveLength(0);
	await expect.element(screen.getByRole('link', { name: 'Heap visualiser' })).toBeInTheDocument();
});

test('a project whose cover is still processing keeps its card', async () => {
	const screen = render(PublicProjectsPage, props({ projects: [props().projects[1]], total: 1 }));

	expect(screen.container.querySelector('li img')).toBe(null);
});

test('the header says which section is being read', async () => {
	const screen = render(PublicProjectsPage, props());

	await expect
		.element(screen.getByRole('navigation').getByText('Projects'))
		.toHaveAttribute('aria-current', 'page');
});

test('the active filter says what it is and how to drop it', async () => {
	const screen = render(PublicProjectsPage, props({ filter: { id: 't-rust', title: 'Rust' } }));

	await expect
		.element(screen.getByRole('link', { name: 'Clear the Rust filter' }))
		.toHaveAttribute('href', '/janedoe/projects');
});

test('an author with no projects gets a page, not a blank', async () => {
	const screen = render(PublicProjectsPage, props({ projects: [], total: 0 }));

	await expect.element(screen.getByText('Nothing published yet.')).toBeInTheDocument();
});

test('never leaks the console to a reader', async () => {
	const screen = render(PublicProjectsPage, props());

	expect(screen.container.innerHTML).not.toMatch(/sign in|\/studio|draft/i);
});

test('has no accessibility violations', async () => {
	render(PublicProjectsPage, props({ filter: { id: 't-rust', title: 'Rust' } }));

	await expectNoA11yViolations(document.body, UNSTYLED_GEOMETRY);
});
