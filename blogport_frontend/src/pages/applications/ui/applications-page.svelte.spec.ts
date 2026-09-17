import { expect, test } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { expectNoA11yViolations } from '$lib/shared/test/a11y';
import ApplicationsPage from './applications-page.svelte';

/**
 * The application tracker.
 *
 * Read-only for now: `PATCH /api/applications/{id}` backs inline status and
 * next action, and adding a job is its own screen. Both are their own slices —
 * this one gives the sidebar's seventh item somewhere to land, with the rows
 * the two lists can actually fill.
 */

const UNSTYLED_GEOMETRY = { rules: { 'target-size': { enabled: false } } };

const rows = [
	{
		id: 'app-1',
		role: 'Senior Backend',
		company: 'Gojek',
		status: { tone: 'inflight' as const, label: 'Interview' },
		nextAction: { text: 'Send the take-home', derived: false },
		applied: '14 Aug',
		cvUsed: 'Backend, sent 14 Aug'
	},
	{
		id: 'app-2',
		role: 'Platform Engineer',
		company: 'Xendit',
		status: { tone: 'neutral' as const, label: 'Draft' },
		nextAction: { text: null, derived: false },
		applied: null,
		cvUsed: 'not chosen yet'
	}
];

test('lists an application per row, with where each has got to', async () => {
	// Screen / Application tracker 12:118: role and company on one line, status,
	// the day it was sent, and what is owed next.
	const screen = render(ApplicationsPage, { rows, failed: false, total: 2, page: 1, perPage: 10 });

	const headers = document.querySelectorAll('thead th');
	expect([...headers].map((th) => th.textContent?.trim())).toEqual([
		'Role & company',
		'Status',
		'CV used',
		'Applied',
		'Next action'
	]);

	const first = screen.getByRole('row').nth(1);
	await expect.element(first.getByText('Senior Backend · Gojek')).toBeInTheDocument();
	await expect.element(first.getByText('Interview')).toBeInTheDocument();
	// The CV cell names the same day ("Backend, sent 14 Aug"), so this asks for
	// the Applied cell rather than for that text wherever it appears.
	await expect
		.element(first.getByRole('cell', { name: '14 Aug', exact: true }))
		.toBeInTheDocument();
	await expect.element(first.getByText('Send the take-home')).toBeInTheDocument();

	await expectNoA11yViolations(document.body, UNSTYLED_GEOMETRY);
});

test('a derived action is plain text until its screen exists', async () => {
	// "Tailor CV" and "Add reflection" lead to screens that are not built, and
	// accent ink on text that goes nowhere promises a link that is not there.
	const screen = render(ApplicationsPage, {
		rows: [{ ...rows[0], nextAction: { text: 'Add reflection', derived: true } }],
		failed: false,
		total: 1,
		page: 1,
		perPage: 10
	});

	await expect.element(screen.getByText('Add reflection').first()).toBeInTheDocument();
	expect(screen.getByRole('link', { name: 'Add reflection' }).elements()).toHaveLength(0);
});

test('a draft and an empty next action are dashes, not blanks', async () => {
	const screen = render(ApplicationsPage, { rows, failed: false, total: 2, page: 1, perPage: 10 });

	const second = screen.getByRole('row').nth(2);
	expect(second.getByText('—', { exact: true }).elements()).toHaveLength(2);
});

test('on a phone each row is a card, with the sent date in its footer', async () => {
	// Mobile / Application tracker 73:120: "Five columns cannot survive 390px."
	const screen = render(ApplicationsPage, { rows, failed: false, total: 2, page: 1, perPage: 10 });

	const cards = screen.getByRole('listitem');
	await expect
		.element(cards.nth(0).getByText('Senior Backend', { exact: true }))
		.toBeInTheDocument();
	await expect.element(cards.nth(0).getByText('Gojek', { exact: true })).toBeInTheDocument();
	await expect.element(cards.nth(0).getByText('sent 14 Aug')).toBeInTheDocument();
	await expect.element(cards.nth(1).getByText('not sent')).toBeInTheDocument();
});

test('empty says what the screen is for and offers the one action', async () => {
	const screen = render(ApplicationsPage, {
		rows: [],
		failed: false,
		total: 0,
		page: 1,
		perPage: 10
	});

	// Screen / Applications — empty 84:701.
	await expect
		.element(screen.getByText('No applications yet', { exact: true }))
		.toBeInTheDocument();
	await expect
		.element(screen.getByText('Paste a job posting and the tracker starts from there.'))
		.toBeInTheDocument();
	// Not "Add a job" twice: the header already says that, and two links with
	// the same name on one screen is a list a screen reader cannot tell apart.
	await expect
		.element(screen.getByRole('link', { name: 'Add your first job' }))
		.toBeInTheDocument();

	await expectNoA11yViolations(document.body, UNSTYLED_GEOMETRY);
});

test('an error says what failed and that nothing was lost', async () => {
	// Never blames the person, and never leaves them wondering whether the
	// applications themselves are gone.
	const screen = render(ApplicationsPage, {
		rows: [],
		failed: true,
		total: 0,
		page: 1,
		perPage: 10
	});

	await expect
		.element(screen.getByRole('heading', { name: 'Couldn’t load your applications' }))
		.toBeInTheDocument();
	await expect
		.element(screen.getByText('Something went wrong on our side. Your applications are safe.'))
		.toBeInTheDocument();

	await expectNoA11yViolations(document.body, UNSTYLED_GEOMETRY);
});

test('it says how many there are, not just how many are shown', async () => {
	const screen = render(ApplicationsPage, {
		rows,
		failed: false,
		total: 24,
		page: 1,
		perPage: 10
	});

	await expect.element(screen.getByText('2 of 24 applications')).toBeInTheDocument();
});

test('a second page can be reached, and the first cannot be gone back past', async () => {
	const asked: number[] = [];
	const screen = render(ApplicationsPage, {
		rows,
		failed: false,
		total: 24,
		page: 1,
		perPage: 10,
		onpage: (n: number) => asked.push(n)
	});

	await expect.element(screen.getByRole('button', { name: 'Previous page' })).toBeDisabled();
	await screen.getByRole('button', { name: 'Next page' }).click();

	expect(asked).toEqual([2]);
});

test('one page of rows needs no pager at all', async () => {
	const screen = render(ApplicationsPage, { rows, failed: false, total: 2, page: 1, perPage: 10 });

	expect(screen.getByRole('button', { name: 'Next page' }).elements()).toHaveLength(0);
});

test('loading is rows, not a spinner, and it is announced', async () => {
	// The fourth of §06's four states. It was missing: this screen had rows,
	// empty and error, and nothing at all in between.
	const screen = render(ApplicationsPage, {
		rows: [],
		failed: false,
		loading: true,
		total: 0,
		page: 1,
		perPage: 10
	});

	await expect.element(screen.getByRole('status')).toHaveTextContent('Loading applications');
	expect(screen.getByText('No applications yet', { exact: true }).elements()).toHaveLength(0);
});

test('each row names the CV that was sent, and when', async () => {
	// Screen / Application tracker 12:118 — "Backend, sent 14 Aug". It comes from
	// the frozen snapshot, so renaming the CV later cannot rewrite this cell.
	const screen = render(ApplicationsPage, { rows, failed: false, total: 2, page: 1, perPage: 10 });

	const table = screen.getByRole('table');
	await expect.element(table.getByText('Backend, sent 14 Aug')).toBeInTheDocument();
	await expect.element(table.getByText('not chosen yet')).toBeInTheDocument();
});
