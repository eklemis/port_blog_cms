import { expect, test } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { expectNoA11yViolations } from '$lib/shared/test/a11y';
import DraftPreviewPage from './draft-preview-page.svelte';

/**
 * `/preview/[token]` — what a reviewer sees.
 *
 * The public post layout with a strip saying this one is not published. The
 * strip is the point: without it a reviewer cannot tell a draft from the live
 * post, and may link to it as though it were public.
 */

const UNSTYLED_GEOMETRY = { rules: { 'target-size': { enabled: false } } };

const post = {
	title: 'Building a CMS in Rust',
	excerpt: 'A walk through the layout.',
	content: 'The API is one Actix Web service.\n\nIt is split into vertical slices.',
	topics: [{ id: 't1', title: 'Rust' }],
	published_at: null
};

test('says this is a draft before it says anything else', async () => {
	const screen = render(DraftPreviewPage, { post });

	await expect.element(screen.getByText('Draft preview')).toBeInTheDocument();
	await expect.element(screen.getByText(/not published/)).toBeInTheDocument();
});

test('reads as the post will read', async () => {
	const screen = render(DraftPreviewPage, { post });

	await expect
		.element(screen.getByRole('heading', { level: 1 }))
		.toHaveTextContent('Building a CMS in Rust');
	await expect.element(screen.getByText('A walk through the layout.')).toBeInTheDocument();
	await expect.element(screen.getByText('Rust', { exact: true })).toBeInTheDocument();
	await expect.element(screen.getByText(/one Actix Web service/)).toBeInTheDocument();
});

test('a body of several paragraphs stays several paragraphs', async () => {
	// Markdown is not rendered yet — see the PR — but the shape of what was
	// written survives, rather than collapsing into one block.
	render(DraftPreviewPage, { post });

	expect(document.querySelectorAll('[data-body] p').length).toBe(2);
});

test('has no accessibility violations', async () => {
	render(DraftPreviewPage, { post });

	await expectNoA11yViolations(document.body, UNSTYLED_GEOMETRY);
});
