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
		nextAction: 'Send the take-home',
		applied: '2 days ago'
	},
	{
		id: 'app-2',
		role: 'Platform Engineer',
		company: 'Xendit',
		status: { tone: 'neutral' as const, label: 'Draft' },
		nextAction: '',
		applied: 'Not sent'
	}
];

test('lists an application per row, with where each has got to', async () => {
	const screen = render(ApplicationsPage, { rows, failed: false });

	await expect.element(screen.getByText('Senior Backend')).toBeInTheDocument();
	await expect.element(screen.getByText('Gojek')).toBeInTheDocument();
	await expect.element(screen.getByText('Interview')).toBeInTheDocument();
	await expect.element(screen.getByText('Send the take-home')).toBeInTheDocument();
	await expect.element(screen.getByText('2 days ago')).toBeInTheDocument();

	await expectNoA11yViolations(document.body, UNSTYLED_GEOMETRY);
});

test('a draft says it has not been sent rather than showing a date', async () => {
	const screen = render(ApplicationsPage, { rows, failed: false });

	await expect.element(screen.getByText('Not sent')).toBeInTheDocument();
});

test('empty says what the screen is for and offers the one action', async () => {
	const screen = render(ApplicationsPage, { rows: [], failed: false });

	await expect.element(screen.getByText('No applications yet.')).toBeInTheDocument();
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
	const screen = render(ApplicationsPage, { rows: [], failed: true });

	await expect.element(screen.getByText("We couldn't load your applications.")).toBeInTheDocument();
	await expect.element(screen.getByText(/Nothing has happened to them/)).toBeInTheDocument();

	await expectNoA11yViolations(document.body, UNSTYLED_GEOMETRY);
});

test('loading is rows, not a spinner, and it is announced', async () => {
	// The fourth of §06's four states. It was missing: this screen had rows,
	// empty and error, and nothing at all in between.
	const screen = render(ApplicationsPage, { rows: [], failed: false, loading: true });

	await expect.element(screen.getByRole('status')).toHaveTextContent('Loading applications');
	expect(screen.getByText('No applications yet.').elements()).toHaveLength(0);
});

test('the table names its columns for a screen reader, not just visually', async () => {
	const screen = render(ApplicationsPage, { rows, failed: false });

	await expect.element(screen.getByRole('columnheader', { name: 'Role' })).toBeInTheDocument();
	await expect.element(screen.getByRole('columnheader', { name: 'Status' })).toBeInTheDocument();
});
