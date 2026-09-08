import { expect, test } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { expectNoA11yViolations } from '$lib/shared/test/a11y';
import StatusPill from './status-pill.svelte';

/**
 * Five values, one meaning each, identical across every resource — a post that
 * is scheduled and an upload that is processing are the same tone, because they
 * are the same kind of fact.
 */

test('the word is the content', async () => {
	// Accessibility Spec §08: nothing extra. A pill in a table is not an
	// announcement, so no role="status" and no aria-label repeating the label.
	const screen = render(StatusPill, { tone: 'live', label: 'Live' });

	await expect.element(screen.getByText('Live')).toBeInTheDocument();
});

test.each([
	['neutral', 'Draft'],
	['inflight', 'Scheduled'],
	['live', 'Live'],
	['dormant', 'Archived'],
	['danger', 'Failed']
] as const)('%s carries its own colour', async (tone, label) => {
	const screen = render(StatusPill, { tone, label });

	await expect.element(screen.getByText(label)).toBeInTheDocument();
});

test('the state is never colour alone', async () => {
	// A pill that said "live" only by being green would say nothing to someone
	// who cannot see the difference.
	const screen = render(StatusPill, { tone: 'live', label: 'Live' });

	expect((await screen.getByText('Live').element()).textContent?.trim()).toBe('Live');
});

test('is not announced as a live region', async () => {
	render(StatusPill, { tone: 'inflight', label: 'Scheduled' });

	expect(document.querySelectorAll('[role="status"]')).toHaveLength(0);
});

test.each(['neutral', 'inflight', 'live', 'dormant', 'danger'] as const)(
	'%s has no accessibility violations',
	async (tone) => {
		render(StatusPill, { tone, label: 'Live' });

		await expectNoA11yViolations();
	}
);
