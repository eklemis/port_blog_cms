import { expect, test } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { expectNoA11yViolations } from '$lib/shared/test/a11y';
import PublicProjectPage from './public-project-page.svelte';

/** Screen / Public project 82:735 · Mobile / Public project 88:2291. */

/** These specs render without the stylesheet, so hit areas are not real. */
const UNSTYLED_GEOMETRY = { rules: { 'target-size': { enabled: false } } };

const props = (over: Record<string, unknown> = {}) => ({
	author: { username: 'janedoe', fullName: 'Jane Doe', avatarSrc: null },
	project: {
		title: 'Blogport CMS',
		techStack: ['Rust', 'Actix Web', 'SeaORM'],
		topics: [
			{ id: 't-rust', title: 'Rust' },
			{ id: 't-systems', title: 'Systems' }
		],
		repoUrl: 'https://github.com/eklemis/port_blog_cms',
		demoUrl: 'https://example.test/demo'
	},
	bodyHtml: '<p>A portfolio CMS across four deployed services.</p>',
	images: [
		{ src: 'https://api.example.test/m/1/large', alt: 'The console' },
		{ src: 'https://api.example.test/m/2/large', alt: 'The editor' },
		{ src: 'https://api.example.test/m/3/large', alt: 'The tracker' }
	],
	...over
});

test('leads with the way back, the name, and what it is', async () => {
	const screen = render(PublicProjectPage, props());

	await expect
		.element(screen.getByRole('link', { name: '← Projects' }))
		.toHaveAttribute('href', '/janedoe/projects');
	await expect
		.element(screen.getByRole('heading', { level: 1, name: 'Blogport CMS' }))
		.toBeInTheDocument();
	await expect
		.element(screen.getByText('A portfolio CMS across four deployed services.'))
		.toBeInTheDocument();
});

test('offers the repository first and the running thing second', async () => {
	// 18:38: "One primary action per screen." The repository is the one an
	// engineer came for; the demo is the secondary.
	const screen = render(PublicProjectPage, props());

	await expect
		.element(screen.getByRole('link', { name: 'View repository' }))
		.toHaveAttribute('href', 'https://github.com/eklemis/port_blog_cms');
	await expect.element(screen.getByRole('link', { name: 'Live demo' })).toBeInTheDocument();
});

test('a project with no repository still offers whatever it has', async () => {
	const screen = render(
		PublicProjectPage,
		props({ project: { ...props().project, repoUrl: null } })
	);

	expect(screen.getByRole('link', { name: 'View repository' }).elements()).toHaveLength(0);
	await expect.element(screen.getByRole('link', { name: 'Live demo' })).toBeInTheDocument();
});

test('the topics are links back to the projects filtered by them', async () => {
	const screen = render(PublicProjectPage, props());

	await expect
		.element(screen.getByRole('link', { name: 'Rust' }))
		.toHaveAttribute('href', '/janedoe/projects?topic_id=t-rust');
});

test('shows the first image large, and the rest as a way to reach them', async () => {
	const screen = render(PublicProjectPage, props());

	await expect.element(screen.getByRole('img', { name: 'The console' })).toBeInTheDocument();
	await expect.element(screen.getByRole('button', { name: 'Show The editor' })).toBeInTheDocument();
});

test('choosing a thumbnail makes it the one on show', async () => {
	// The frame marks one thumbnail with a 2px accent border (82:772). A marked
	// thumbnail that does nothing would be a mark that means nothing.
	const screen = render(PublicProjectPage, props());

	await screen.getByRole('button', { name: 'Show The tracker' }).click();

	await expect.element(screen.getByRole('img', { name: 'The tracker' })).toBeInTheDocument();
	await expect
		.element(screen.getByRole('button', { name: 'Show The tracker' }))
		.toHaveAttribute('aria-current', 'true');
});

test('one image is a picture, not a gallery', async () => {
	const screen = render(PublicProjectPage, props({ images: [props().images[0]] }));

	await expect.element(screen.getByRole('img', { name: 'The console' })).toBeInTheDocument();
	expect(screen.getByRole('button', { name: /^Show/ }).elements()).toHaveLength(0);
});

test('a project with no images at all is still a project', async () => {
	const screen = render(PublicProjectPage, props({ images: [] }));

	await expect
		.element(screen.getByRole('heading', { level: 1, name: 'Blogport CMS' }))
		.toBeInTheDocument();
	expect(screen.container.querySelector('main img, article img')).toBe(null);
});

test('never leaks the console to a reader', async () => {
	const screen = render(PublicProjectPage, props());

	expect(screen.container.innerHTML).not.toMatch(/sign in|\/studio|draft/i);
});

test('has no accessibility violations', async () => {
	render(PublicProjectPage, props());

	await expectNoA11yViolations(document.body, UNSTYLED_GEOMETRY);
});
