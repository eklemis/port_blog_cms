import { expect, test } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { userEvent } from '@vitest/browser/context';
import { expectNoA11yViolations } from '$lib/shared/test/a11y';
import ConfirmDialog from './confirm-dialog.svelte';

/**
 * The third rung of destruction — §06: "a dialog naming the item, requiring the
 * title to be typed, with a danger-coloured button."
 *
 * Four states from the Forms Spec: idle, typing, armed, working. The rules are
 * all about what cannot happen by accident: it opens on Cancel, Enter never
 * confirms, and nothing confirms until the typed string matches exactly.
 */

const UNSTYLED_GEOMETRY = { rules: { 'target-size': { enabled: false } } };

const props = (over: Record<string, unknown> = {}) => ({
	open: true,
	title: 'Delete this post forever?',
	consequence: 'It cannot be restored, and its public address stops working.',
	match: 'Building a CMS',
	confirmLabel: 'Delete forever',
	onconfirm: () => {},
	oncancel: () => {},
	...over
});

test('shut, it is not there at all', async () => {
	const screen = render(ConfirmDialog, props({ open: false }));

	expect(screen.getByRole('dialog').elements()).toHaveLength(0);
});

test('open, it names what it is about to destroy and what that costs', async () => {
	const screen = render(ConfirmDialog, props());

	const dialog = screen.getByRole('dialog', { name: 'Delete this post forever?' });
	await expect.element(dialog).toBeInTheDocument();
	await expect
		.element(dialog)
		.toHaveAccessibleDescription('It cannot be restored, and its public address stops working.');
	await expect.element(screen.getByText('Building a CMS', { exact: true })).toBeInTheDocument();
});

test('it opens with focus on Cancel, never on the button that destroys', async () => {
	const screen = render(ConfirmDialog, props());

	await expect.element(screen.getByRole('button', { name: 'Cancel' })).toHaveFocus();
});

test('the destructive button stays shut until the name is typed exactly', async () => {
	const screen = render(ConfirmDialog, props());
	const confirm = screen.getByRole('button', { name: 'Delete forever' });
	const input = screen.getByRole('textbox');

	await expect.element(confirm).toBeDisabled();

	await input.fill('Building a CM');
	await expect.element(confirm).toBeDisabled();

	// Exactly: a case difference is a different name, and "nearly" is the
	// wrong standard for something that cannot be undone.
	await input.fill('building a cms');
	await expect.element(confirm).toBeDisabled();

	await input.fill('Building a CMS');
	await expect.element(confirm).toBeEnabled();
});

test('armed, pressing the button is what confirms', async () => {
	const confirmed: boolean[] = [];
	const screen = render(ConfirmDialog, props({ onconfirm: () => confirmed.push(true) }));

	await screen.getByRole('textbox').fill('Building a CMS');
	await screen.getByRole('button', { name: 'Delete forever' }).click();

	expect(confirmed).toEqual([true]);
});

test('Enter never confirms, even armed', async () => {
	// The Forms Spec's own words. A habit of pressing Enter after typing is
	// exactly the reflex this dialog exists to interrupt.
	const confirmed: boolean[] = [];
	const screen = render(ConfirmDialog, props({ onconfirm: () => confirmed.push(true) }));

	await screen.getByRole('textbox').fill('Building a CMS');
	await userEvent.keyboard('{Enter}');

	expect(confirmed).toHaveLength(0);
});

test('Escape cancels', async () => {
	const cancelled: boolean[] = [];
	render(ConfirmDialog, props({ oncancel: () => cancelled.push(true) }));

	await userEvent.keyboard('{Escape}');

	expect(cancelled).toEqual([true]);
});

test('working, nothing can be pressed twice and it says it is working', async () => {
	// Purges wait for the server — optimism about an irreversible action is
	// just an inaccurate screen.
	// Typed first, then working — the order it happens in. The input locks
	// once the call is in the air, so it cannot be typed into afterwards.
	const screen = render(ConfirmDialog, props());
	await screen.getByRole('textbox').fill('Building a CMS');
	await screen.rerender(props({ working: true }));

	await expect
		.element(screen.getByRole('button', { name: 'Delete forever' }))
		.toHaveAttribute('aria-busy', 'true');
	await expect.element(screen.getByRole('button', { name: 'Cancel' })).toBeDisabled();
});

test('has no accessibility violations', async () => {
	render(ConfirmDialog, props());

	await expectNoA11yViolations(document.body, UNSTYLED_GEOMETRY);
});
