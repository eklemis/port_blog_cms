import { expect, test } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { expectNoA11yViolations } from '$lib/shared/test/a11y';
import AssistCard from './assist-card.svelte';

/**
 * The editor rail's Assist card — Screen / AI states 71:2, the first of the
 * three generation states.
 *
 * Only the off state is built. The working companion (selection toolbar,
 * suggestion card, Apply · Skip) needs a model call on a passage, and no
 * endpoint takes one.
 */

/** These specs render without the stylesheet, so hit areas are not real. */
const UNSTYLED_GEOMETRY = { rules: { 'target-size': { enabled: false } } };

test('says assistance is off, in the words the frame chose', async () => {
	const screen = render(AssistCard, {});

	await expect
		.element(screen.getByText('Writing assistance is not enabled on this deployment.'))
		.toBeInTheDocument();
	await expect
		.element(screen.getByText('Everything else works normally. Nothing here is broken.'))
		.toBeInTheDocument();
});

test('offers nothing to press, because pressing it could not help', async () => {
	// 71:2: "503 AI_DISABLED. A deployment with no provider key is a valid
	// configuration, not a fault — so there is no Retry, because retrying cannot
	// help."
	const screen = render(AssistCard, {});

	expect(screen.getByRole('button').elements()).toHaveLength(0);
	expect(screen.getByRole('link').elements()).toHaveLength(0);
});

test('is muted, never red', async () => {
	// The same annotation: "Muted, never red." A configuration is not an error,
	// and colouring it like one would send an author looking for a fault.
	const screen = render(AssistCard, {});

	expect(screen.container.innerHTML).not.toMatch(/st-danger|text-red|border-red/);
});

test('names itself so the rail reads as a rail', async () => {
	const screen = render(AssistCard, {});

	await expect.element(screen.getByRole('region', { name: 'Assist' })).toBeInTheDocument();
});

test('has no accessibility violations', async () => {
	render(AssistCard, {});

	await expectNoA11yViolations(document.body, UNSTYLED_GEOMETRY);
});
