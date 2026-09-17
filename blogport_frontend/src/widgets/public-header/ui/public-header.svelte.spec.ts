import { expect, test } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { expectNoA11yViolations } from '$lib/shared/test/a11y';
import PublicHeader from './public-header.svelte';

/** These specs render without the stylesheet, so hit areas are not real. */
const UNSTYLED_GEOMETRY = { rules: { 'target-size': { enabled: false } } };

/**
 * Screen / Public post 72:3 and Mobile / Public post 73:293 — the header every
 * public page shares, keyed on the author.
 */

const props = (over: Record<string, unknown> = {}) => ({
	username: 'janedoe',
	fullName: 'Jane Doe',
	avatarSrc: 'https://api.example.test/api/public/media/av-1/thumbnail',
	...over
});

test('introduces the author whose work this is', async () => {
	const screen = render(PublicHeader, props());

	await expect.element(screen.getByText('Jane Doe')).toBeInTheDocument();
	await expect.element(screen.getByRole('img', { name: 'Jane Doe' })).toBeInTheDocument();
});

test('an author with no avatar still has a header, not a broken image', async () => {
	const screen = render(PublicHeader, props({ avatarSrc: null }));

	await expect.element(screen.getByText('Jane Doe')).toBeInTheDocument();
	expect(screen.container.querySelectorAll('img')).toHaveLength(0);
});

test('carries the author’s other work, keyed on their name', async () => {
	// §03 public surfaces: the listings share a header, so any entry point leads
	// to the other two.
	const screen = render(PublicHeader, props());
	const nav = screen.getByRole('navigation');

	await expect
		.element(nav.getByRole('link', { name: 'Writing' }))
		.toHaveAttribute('href', '/janedoe/blog');
	await expect
		.element(nav.getByRole('link', { name: 'Projects' }))
		.toHaveAttribute('href', '/janedoe/projects');
});

test('escapes a username rather than trusting it into a path', async () => {
	const screen = render(PublicHeader, props({ username: 'jane doe' }));

	await expect
		.element(screen.getByRole('navigation').getByRole('link', { name: 'Writing' }))
		.toHaveAttribute('href', '/jane%20doe/blog');
});

test('offers no way into the console — the reader is not a lapsed author', async () => {
	// §03: "No edit affordances, no draft indicators, no 'Sign in' nag over the
	// content."
	const screen = render(PublicHeader, props());

	expect(screen.container.innerHTML).not.toMatch(/sign in|studio|edit/i);
});

test('has no accessibility violations', async () => {
	render(PublicHeader, props());

	await expectNoA11yViolations(document.body, UNSTYLED_GEOMETRY);
});
