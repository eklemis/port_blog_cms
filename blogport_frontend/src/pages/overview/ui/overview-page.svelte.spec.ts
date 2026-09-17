import { beforeEach, expect, test } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { expectNoA11yViolations } from '$lib/shared/test/a11y';
import OverviewPage from './overview-page.svelte';

/**
 * The console's front door.
 *
 * Four tiles and the first-run checklist. The tiles have no sub-lines and there
 * is no "Needs attention": both need per-status counts that no endpoint carries,
 * which §09 of the blueprint says in as many words.
 */

/**
 * The checklist's links and its dismiss button are laid out by Tailwind, and
 * these specs render without the stylesheet — so axe measures the browser's
 * default box rather than the one that ships. Geometry is checked at the three
 * widths by hand; everything else axe knows about stays on.
 */
const UNSTYLED_GEOMETRY = { rules: { 'target-size': { enabled: false } } };

const counts = { posts: 24, projects: 8, resumes: 3, applications: 5, topics: 6 };

/** A fresh account: nothing made yet, nothing dismissed. */
const first = { posts: 0, projects: 0, resumes: 0, applications: 0, topics: 0 };

beforeEach(() => localStorage.clear());

test('greets the person by the name they go by', async () => {
	const screen = render(OverviewPage, { fullName: 'Jane Doe', counts });

	await expect.element(screen.getByRole('heading', { level: 1 })).toHaveTextContent(/, Jane$/);
});

test('shows the counts it was given, all four of them', async () => {
	const screen = render(OverviewPage, { fullName: 'Jane Doe', counts });

	for (const [label, value] of [
		['Posts', '24'],
		['Projects', '8'],
		['Résumés', '3'],
		['Applications', '5']
	]) {
		await expect.element(screen.getByText(label, { exact: true })).toBeInTheDocument();
		await expect.element(screen.getByText(value, { exact: true })).toBeInTheDocument();
	}
});

test('a count that could not be fetched is not reported as none', async () => {
	// Zero posts and "we could not ask" are different facts, and only one of
	// them should make someone go looking for their missing work.
	const screen = render(OverviewPage, {
		fullName: 'Jane Doe',
		counts: { ...counts, posts: null }
	});

	await expect.element(screen.getByText('—')).toBeInTheDocument();
	expect(screen.getByText('0', { exact: true }).elements()).toHaveLength(0);
});

test('a real zero is still a zero', async () => {
	const screen = render(OverviewPage, { fullName: 'Jane Doe', counts: first });

	expect(screen.getByText('0', { exact: true }).elements().length).toBeGreaterThan(0);
	expect(screen.getByText('—').elements()).toHaveLength(0);
});

// ── first run ──────────────────────────────────────────────────────────────

test('a new account is told the three things to do', async () => {
	const screen = render(OverviewPage, { fullName: 'Jane Doe', counts: first });

	await expect.element(screen.getByText('Getting started')).toBeInTheDocument();
	await expect.element(screen.getByRole('link', { name: 'Create a topic' })).toBeInTheDocument();
	await expect
		.element(screen.getByRole('link', { name: 'Write your first post' }))
		.toBeInTheDocument();
	await expect.element(screen.getByRole('link', { name: 'Add a résumé' })).toBeInTheDocument();
});

test('an account that has done two things is not shown it at all', async () => {
	// It is not a permanent fixture; it goes for good.
	const screen = render(OverviewPage, { fullName: 'Jane Doe', counts });

	expect(screen.getByText('Getting started').elements()).toHaveLength(0);
});

test('dismissing it puts it away, and it stays away', async () => {
	const screen = render(OverviewPage, { fullName: 'Jane Doe', counts: first });

	await screen.getByRole('button', { name: 'Dismiss' }).click();
	expect(screen.getByText('Getting started').elements()).toHaveLength(0);

	// A reload is a fresh render reading the same browser.
	const again = render(OverviewPage, { fullName: 'Jane Doe', counts: first });
	expect(again.getByText('Getting started').elements()).toHaveLength(0);
});

test('what is already done is ticked rather than asked for again', async () => {
	const screen = render(OverviewPage, {
		fullName: 'Jane Doe',
		counts: { ...first, topics: 2 }
	});

	await expect.element(screen.getByText('Create a topic')).toHaveAttribute('data-done', 'true');
});

// ── Screen / Overview 68:2 ─────────────────────────────────────────────────

test('the posts tile says how the posts divide, when the summary came back', async () => {
	// "12 live · 9 drafts · 3 archived" — the one sub-line now backed, by
	// /api/blog/summary. The other three tiles stay without one until theirs is.
	const screen = render(OverviewPage, {
		fullName: 'Jane Doe',
		counts: { ...counts, postStates: { live: 12, drafts: 9, archived: 3 } }
	});

	await expect.element(screen.getByText('12 live · 9 drafts · 3 archived')).toBeInTheDocument();
});

test('without the summary the posts tile says nothing beneath the number', async () => {
	// A sub-line of dashes would read as nothing live, nothing drafted.
	const screen = render(OverviewPage, {
		fullName: 'Jane Doe',
		counts: { ...counts, postStates: null }
	});

	expect(screen.getByText(/ live · /).elements()).toHaveLength(0);
});

test('the checklist carries no footnote about itself', async () => {
	// The sentence inside the card was a note on the wrong layer, and the
	// designer has deleted it.
	const screen = render(OverviewPage, { fullName: 'Jane Doe', counts: first });

	expect(screen.getByText(/not a permanent fixture/).elements()).toHaveLength(0);
});

test('a ticked item says it is done to someone who cannot see the tick', async () => {
	const screen = render(OverviewPage, { fullName: 'Jane Doe', counts: { ...first, topics: 2 } });

	await expect
		.element(screen.getByRole('link', { name: 'Done: Create a topic' }))
		.toBeInTheDocument();
	await expect
		.element(screen.getByRole('link', { name: 'Write your first post' }))
		.toBeInTheDocument();
});

test('has no accessibility violations', async () => {
	render(OverviewPage, { fullName: 'Jane Doe', counts });

	await expectNoA11yViolations();
});

test('the first-run checklist has none either', async () => {
	render(OverviewPage, { fullName: 'Jane Doe', counts: first });

	await expectNoA11yViolations(document.body, UNSTYLED_GEOMETRY);
});
