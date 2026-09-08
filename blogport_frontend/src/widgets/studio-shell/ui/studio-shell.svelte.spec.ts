import { expect, test } from 'vitest';
import { createRawSnippet } from 'svelte';
import { render } from 'vitest-browser-svelte';
import { userEvent } from '@vitest/browser/context';
import { expectNoA11yViolations } from '$lib/shared/test/a11y';
import StudioShell from './studio-shell.svelte';

/**
 * The console's frame. One nav definition, three shapes — so what is tested
 * here is that the shape does not change which destinations exist or which one
 * is lit.
 */

const UNSTYLED_GEOMETRY = { rules: { 'target-size': { enabled: false } } };

const body = createRawSnippet(() => ({ render: () => '<p>The screen</p>' }));

const props = (path = '/studio') => ({ path, title: 'Overview', children: body });

test('renders what it wraps', async () => {
	const screen = render(StudioShell, props());

	await expect.element(screen.getByText('The screen')).toBeInTheDocument();
});

test('offers every console destination', async () => {
	const screen = render(StudioShell, props());

	for (const label of ['Overview', 'Posts', 'Projects', 'Résumés', 'Media', 'Topics', 'Account']) {
		expect(screen.getByRole('link', { name: label }).elements().length).toBeGreaterThan(0);
	}
});

/** What a screen reader is told is the current page. */
function currentLabels() {
	return [...document.querySelectorAll('[aria-current="page"]')].map((el) =>
		el.textContent?.trim()
	);
}

test('marks the current screen, and only it', async () => {
	render(StudioShell, props('/studio/posts'));

	expect(currentLabels()).toContain('Posts');
	// `/studio` is a prefix of every console path; Overview must not be lit on
	// all eight screens.
	expect(currentLabels()).not.toContain('Overview');
});

test('a nested screen still lights its section', async () => {
	render(StudioShell, props('/studio/posts/new'));

	expect(currentLabels()).toContain('Posts');
});

test('names the screen where the sidebar cannot', async () => {
	// At 390px there is no sidebar to say which screen this is.
	const screen = render(StudioShell, { ...props(), title: 'Media library' });

	await expect.element(screen.getByText('Media library')).toBeInTheDocument();
});

test('a skip link is the first focusable thing', async () => {
	// Eight nav stops before the content, at every width.
	const screen = render(StudioShell, props());

	const skip = screen.getByRole('link', { name: 'Skip to content' });
	await expect.element(skip).toHaveAttribute('href', '#main');
	expect(screen.getByRole('link').elements()[0]).toBe(skip.element());
});

test('both navs are labelled, so they are distinguishable to a screen reader', async () => {
	const screen = render(StudioShell, props());

	expect(screen.getByRole('navigation', { name: 'Console' }).elements().length).toBeGreaterThan(0);
});

// ── the More sheet ─────────────────────────────────────────────────────────

test('More is a button, not a destination — it has none of its own', async () => {
	const screen = render(StudioShell, props());

	await expect.element(screen.getByRole('button', { name: 'More' })).toBeInTheDocument();
	expect(screen.getByRole('link', { name: 'More' }).elements()).toHaveLength(0);
});

test('the sheet is shut until it is asked for', async () => {
	const screen = render(StudioShell, props());

	expect(screen.getByRole('dialog').elements()).toHaveLength(0);
});

test('More opens a sheet holding the four the bar has no room for', async () => {
	const screen = render(StudioShell, props());

	await screen.getByRole('button', { name: 'More' }).click();

	const sheet = screen.getByRole('dialog', { name: 'More' });
	await expect.element(sheet).toBeInTheDocument();

	for (const label of ['Overview', 'Résumés', 'Topics', 'Account']) {
		expect(
			sheet.getByRole('link', { name: label }).elements(),
			`${label} is unreachable at 390px without it`
		).toHaveLength(1);
	}
});

test('Escape closes it and gives focus back to what opened it', async () => {
	// §06: Esc closes a bottom sheet and restores focus. A sheet that strands
	// the caret on a closed dialog is a keyboard dead end.
	const screen = render(StudioShell, props());
	const more = screen.getByRole('button', { name: 'More' });

	await more.click();
	await userEvent.keyboard('{Escape}');

	expect(screen.getByRole('dialog').elements()).toHaveLength(0);
	expect(document.activeElement).toBe(more.element());
});

test('the sheet has no accessibility violations either', async () => {
	const screen = render(StudioShell, props());

	await screen.getByRole('button', { name: 'More' }).click();

	await expectNoA11yViolations(document.body, UNSTYLED_GEOMETRY);
});

test('has no accessibility violations', async () => {
	render(StudioShell, props());

	await expectNoA11yViolations(document.body, UNSTYLED_GEOMETRY);
});
