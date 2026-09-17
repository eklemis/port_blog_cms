import { expect, test } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { expectNoA11yViolations } from '$lib/shared/test/a11y';
import PublicPostPage from './public-post-page.svelte';

/** Screen / Public post 72:2 · Mobile / Public post 73:292. */

/** These specs render without the stylesheet, so hit areas are not real. */
const UNSTYLED_GEOMETRY = { rules: { 'target-size': { enabled: false } } };

const props = (over: Record<string, unknown> = {}) => ({
	author: {
		username: 'janedoe',
		fullName: 'Jane Doe',
		avatarSrc: 'https://api.example.test/api/public/media/av-1/thumbnail'
	},
	post: {
		title: 'Building a CMS in Rust',
		excerpt: 'A walk through the hexagonal layout, and the one rule the build does not enforce.',
		publishedAt: '2026-08-14T09:30:00Z'
	},
	topics: [
		{ id: 't-rust', title: 'Rust' },
		{ id: 't-systems', title: 'Systems' }
	],
	cover: {
		src: 'https://api.example.test/api/public/media/cov-1/large',
		alt: 'Hexagonal layout of the API'
	},
	bodyHtml: '<h2>Layout</h2><p>One service, split into slices.</p>',
	readMinutes: 7,
	...over
});

test('leads with what the frame leads with — the date, the read, the title', async () => {
	const screen = render(PublicPostPage, props());

	await expect
		.element(screen.getByRole('heading', { level: 1, name: 'Building a CMS in Rust' }))
		.toBeInTheDocument();
	// Spelled by Intl in the reader's own locale, so the expectation is too —
	// hard-coding "14 August 2026" would only assert where the runner happens
	// to live.
	const on = new Intl.DateTimeFormat(undefined, {
		day: 'numeric',
		month: 'long',
		year: 'numeric'
	}).format(new Date('2026-08-14T09:30:00Z'));
	await expect.element(screen.getByText(on)).toBeInTheDocument();
	await expect.element(screen.getByText('7 min read')).toBeInTheDocument();
});

test('the body arrives as markup, already made', async () => {
	// The parser ran on the server. What reaches the reader is the article.
	const screen = render(PublicPostPage, props());

	await expect
		.element(screen.getByRole('heading', { level: 2, name: 'Layout' }))
		.toBeInTheDocument();
	await expect.element(screen.getByText('One service, split into slices.')).toBeInTheDocument();
});

test('the topic chips are real links, not labels', async () => {
	// §03: "Render topic chips as links" — both public listings accept topic_id.
	const screen = render(PublicPostPage, props());

	await expect
		.element(screen.getByRole('link', { name: 'Rust' }))
		.toHaveAttribute('href', '/janedoe/blog?topic_id=t-rust');
});

test('the cover carries the alt text its author wrote', async () => {
	const screen = render(PublicPostPage, props());

	await expect
		.element(screen.getByRole('img', { name: 'Hexagonal layout of the API' }))
		.toBeInTheDocument();
});

test('a post whose cover is still processing has no cover, not a gap', async () => {
	// A media row exists before its variants do.
	const screen = render(PublicPostPage, props({ cover: null }));

	await expect
		.element(screen.getByRole('heading', { level: 1, name: 'Building a CMS in Rust' }))
		.toBeInTheDocument();
	expect(screen.container.querySelector('article img')).toBe(null);
});

test('a post with no excerpt does not print an empty line where one would be', async () => {
	const screen = render(PublicPostPage, props({ post: { ...props().post, excerpt: null } }));

	expect(screen.container.querySelector('[data-excerpt]')).toBe(null);
});

test('never leaks the console to a reader', async () => {
	// §03: no edit affordances, no draft indicators, no "Sign in" nag.
	const screen = render(PublicPostPage, props());

	expect(screen.container.innerHTML).not.toMatch(/sign in|\/studio|draft/i);
});

test('has no accessibility violations', async () => {
	render(PublicPostPage, props());

	await expectNoA11yViolations(document.body, UNSTYLED_GEOMETRY);
});
