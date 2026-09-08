import { expect, test } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { expectNoA11yViolations } from '$lib/shared/test/a11y';
import SaveIndicator from './save-indicator.svelte';

/**
 * The one place that reports save state — J4 is explicit that it is the only
 * one. Four states in a fixed slot, so nothing below it moves as they change.
 */

const UNSTYLED_GEOMETRY = { rules: { 'target-size': { enabled: false } } };

const at = new Date('2026-09-08T10:42:00').getTime();

test('a settled editor names the time it last saved', async () => {
	const screen = render(SaveIndicator, { state: 'saved', savedAt: at });

	await expect.element(screen.getByText(/^Saved /)).toBeInTheDocument();
});

test('before the first save there is no time to name', async () => {
	// "Saved —" would be worse than saying nothing.
	const screen = render(SaveIndicator, { state: 'saved', savedAt: null });

	await expect.element(screen.getByText('Saved')).toBeInTheDocument();
});

test('the other three say what is happening', async () => {
	for (const [state, text] of [
		['unsaved', 'Unsaved changes'],
		['saving', 'Saving…'],
		['retrying', "Couldn't save — retrying"]
	] as const) {
		const screen = render(SaveIndicator, { state, savedAt: at });
		await expect.element(screen.getByText(text)).toBeInTheDocument();
	}
});

test('only the failure is coloured as one', async () => {
	// Amber or red on "Saving…" would make an ordinary keystroke look like a
	// problem. Only the state that is actually a problem gets the colour.
	const failing = render(SaveIndicator, { state: 'retrying', savedAt: at });
	await expect.element(failing.getByText("Couldn't save — retrying")).toHaveClass(/text-st-danger/);

	const saving = render(SaveIndicator, { state: 'saving', savedAt: at });
	expect(saving.getByText('Saving…').element().className).not.toMatch(/text-st-danger/);
});

test('it is announced politely, not read out on every keystroke', async () => {
	// A live region that interrupts on each debounce tick is the most likely
	// defect in this product, per the accessibility spec's own note.
	render(SaveIndicator, { state: 'saving', savedAt: at });

	const region = document.querySelector('[role="status"]');
	expect(region).not.toBeNull();
	expect(document.querySelector('[role="alert"]')).toBeNull();
});

test('has no accessibility violations', async () => {
	render(SaveIndicator, { state: 'retrying', savedAt: at });

	await expectNoA11yViolations(document.body, UNSTYLED_GEOMETRY);
});
