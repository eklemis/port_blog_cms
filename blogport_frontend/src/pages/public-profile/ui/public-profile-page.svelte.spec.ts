import { expect, test } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { expectNoA11yViolations } from '$lib/shared/test/a11y';
import PublicProfilePage from './public-profile-page.svelte';

/** Screen / Public profile 82:420 · Mobile / Public profile 88:2062. */

/** These specs render without the stylesheet, so hit areas are not real. */
const UNSTYLED_GEOMETRY = { rules: { 'target-size': { enabled: false } } };

const props = (over: Record<string, unknown> = {}) => ({
	author: {
		username: 'janedoe',
		fullName: 'Jane Doe',
		bio: 'Backend engineer, mostly Rust.',
		avatarSrc: 'https://api.example.test/api/public/media/av-1/small'
	},
	projects: [
		{
			slug: 'blogport-cms',
			title: 'Blogport CMS',
			description: 'A portfolio CMS across four services.',
			techStack: ['Rust', 'SvelteKit'],
			cover: { src: 'https://api.example.test/m/1/large', alt: '' }
		}
	],
	posts: [
		{ slug: 'building-a-cms-in-rust', title: 'Building a CMS in Rust', published: '14 Aug 2026' }
	],
	...over
});

test('introduces the person, and says it in one heading', async () => {
	const screen = render(PublicProfilePage, props());

	await expect
		.element(screen.getByRole('heading', { level: 1, name: 'Jane Doe' }))
		.toBeInTheDocument();
	await expect.element(screen.getByText('Backend engineer, mostly Rust.')).toBeInTheDocument();
});

test('is a doorway: each strip leads to the list behind it', async () => {
	// 82:500: "A doorway, not a fourth list: every block here leads somewhere
	// that already exists."
	const screen = render(PublicProfilePage, props());

	await expect
		.element(screen.getByRole('link', { name: 'All projects →' }))
		.toHaveAttribute('href', '/janedoe/projects');
	await expect
		.element(screen.getByRole('link', { name: 'All posts →' }))
		.toHaveAttribute('href', '/janedoe/blog');
});

test('each piece of work links to its own page', async () => {
	const screen = render(PublicProfilePage, props());

	await expect
		.element(screen.getByRole('link', { name: 'Blogport CMS' }))
		.toHaveAttribute('href', '/janedoe/projects/blogport-cms');
	await expect
		.element(screen.getByRole('link', { name: 'Building a CMS in Rust' }))
		.toHaveAttribute('href', '/janedoe/blog/building-a-cms-in-rust');
});

test('names the two sections the way the frame labels them', async () => {
	const screen = render(PublicProfilePage, props());

	await expect.element(screen.getByText('Selected work')).toBeInTheDocument();
	await expect.element(screen.getByText('Recent writing')).toBeInTheDocument();
});

test('an author with no projects yet drops that strip rather than emptying it', async () => {
	// A doorway with a door to nowhere is worse than one door fewer.
	const screen = render(PublicProfilePage, props({ projects: [] }));

	expect(screen.getByText('Selected work').elements()).toHaveLength(0);
	await expect.element(screen.getByText('Recent writing')).toBeInTheDocument();
});

test('an author with nothing at all is still a profile', async () => {
	const screen = render(PublicProfilePage, props({ projects: [], posts: [] }));

	await expect
		.element(screen.getByRole('heading', { level: 1, name: 'Jane Doe' }))
		.toBeInTheDocument();
	expect(screen.getByText('Recent writing').elements()).toHaveLength(0);
});

test('an author with no bio is not given an empty line where one would be', async () => {
	const screen = render(PublicProfilePage, props({ author: { ...props().author, bio: null } }));

	expect(screen.container.querySelector('[data-bio]')).toBe(null);
});

test('never leaks the console to a reader', async () => {
	const screen = render(PublicProfilePage, props());

	expect(screen.container.innerHTML).not.toMatch(/sign in|\/studio|draft/i);
});

test('has no accessibility violations', async () => {
	render(PublicProfilePage, props());

	await expectNoA11yViolations(document.body, UNSTYLED_GEOMETRY);
});
