import { expect, test } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { expectNoA11yViolations } from '$lib/shared/test/a11y';
import OverviewPage from './overview-page.svelte';

/**
 * The console's front door. Three tiles, not the frame's four — see the PR for
 * why "Applications" has no route to belong to.
 */

const counts = { posts: 24, projects: 8, resumes: 3 };

test('greets the person by the name they go by', async () => {
	const screen = render(OverviewPage, { fullName: 'Jane Doe', counts });

	await expect.element(screen.getByRole('heading', { level: 1 })).toHaveTextContent(/, Jane$/);
});

test('shows the counts it was given', async () => {
	const screen = render(OverviewPage, { fullName: 'Jane Doe', counts });

	for (const [label, value] of [
		['Posts', '24'],
		['Projects', '8'],
		['Résumés', '3']
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
		counts: { posts: null, projects: 8, resumes: 3 }
	});

	await expect.element(screen.getByText('—')).toBeInTheDocument();
	expect(screen.getByText('0', { exact: true }).elements()).toHaveLength(0);
});

test('a real zero is still a zero', async () => {
	const screen = render(OverviewPage, {
		fullName: 'Jane Doe',
		counts: { posts: 0, projects: 0, resumes: 0 }
	});

	expect(screen.getByText('0', { exact: true }).elements().length).toBeGreaterThan(0);
	expect(screen.getByText('—').elements()).toHaveLength(0);
});

test('has no accessibility violations', async () => {
	render(OverviewPage, { fullName: 'Jane Doe', counts });

	await expectNoA11yViolations();
});
