import { expect, test } from 'vitest';
import { createRawSnippet } from 'svelte';
import { render } from 'vitest-browser-svelte';
import { userEvent } from '@vitest/browser/context';
import { expectNoA11yViolations } from '$lib/shared/test/a11y';
import Menu from './menu.svelte';

/**
 * The ⋯ menu at the end of a row, and at the end of the editor's top bar.
 *
 * §06 puts Archive here rather than on the row itself: archiving is one click,
 * and a click target that archives by accident is the thing this hides behind.
 */

const UNSTYLED_GEOMETRY = { rules: { 'target-size': { enabled: false } } };

/**
 * A menu's items are the caller's, and closing is theirs to call: `close` is
 * handed to the snippet, and an item calls it when chosen.
 */
const items = createRawSnippet((close: () => () => void) => ({
	render: () => '<button type="button" role="menuitem" data-close>Archive</button>',
	setup: (node: Element) => {
		const fn = close();
		node.addEventListener('click', () => fn());
	}
}));

test('it is shut until it is asked for', async () => {
	const screen = render(Menu, { label: 'More for Building a CMS', items });

	expect(screen.getByRole('menu').elements()).toHaveLength(0);
	await expect
		.element(screen.getByRole('button', { name: 'More for Building a CMS' }))
		.toHaveAttribute('aria-expanded', 'false');
});

test('opening it says so, and shows what is in it', async () => {
	const screen = render(Menu, { label: 'More for Building a CMS', items });

	await screen.getByRole('button', { name: 'More for Building a CMS' }).click();

	await expect.element(screen.getByRole('menu')).toBeInTheDocument();
	await expect.element(screen.getByRole('menuitem', { name: 'Archive' })).toBeInTheDocument();
	await expect
		.element(screen.getByRole('button', { name: 'More for Building a CMS' }))
		.toHaveAttribute('aria-expanded', 'true');
});

test('Escape shuts it and gives focus back to what opened it', async () => {
	const screen = render(Menu, { label: 'More for Building a CMS', items });
	const trigger = screen.getByRole('button', { name: 'More for Building a CMS' });

	await trigger.click();
	await userEvent.keyboard('{Escape}');

	expect(screen.getByRole('menu').elements()).toHaveLength(0);
	expect(document.activeElement).toBe(trigger.element());
});

test('choosing something shuts it, so the row is not left covered', async () => {
	const screen = render(Menu, { label: 'More for Building a CMS', items });

	await screen.getByRole('button', { name: 'More for Building a CMS' }).click();
	await screen.getByRole('menuitem', { name: 'Archive' }).click();

	expect(screen.getByRole('menu').elements()).toHaveLength(0);
});

test('has no accessibility violations', async () => {
	const screen = render(Menu, { label: 'More for Building a CMS', items });
	await screen.getByRole('button', { name: 'More for Building a CMS' }).click();

	await expectNoA11yViolations(document.body, UNSTYLED_GEOMETRY);
});
