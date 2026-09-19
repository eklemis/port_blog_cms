import { expect, test, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { expectNoA11yViolations } from '$lib/shared/test/a11y';
import LanguagePill from './language-pill.svelte';

/**
 * The "EN ▾" pill — Career Studio §02, drawn in 34 places across two shells.
 *
 * It does not appear while the product speaks one language. A switcher that
 * changes a label and nothing else is the defect the Assist card refused to
 * be, one screen over.
 */

/** These specs render without the stylesheet, so hit areas are not real. */
const UNSTYLED_GEOMETRY = { rules: { 'target-size': { enabled: false } } };

const both = ['en', 'id'] as const;

test('stays away while there is only one language to offer', async () => {
	const screen = render(LanguagePill, { locale: 'en', offered: ['en'] });

	// Svelte leaves an anchor comment behind an `{#if}`; what matters is that
	// there is nothing to press.
	expect(screen.getByRole('button').elements()).toHaveLength(0);
});

test('names the language it is in, not the one it would switch to', async () => {
	const screen = render(LanguagePill, { locale: 'en', offered: both });

	await expect.element(screen.getByRole('button', { name: /Language/ })).toHaveTextContent('EN');
});

test('offers every language the product can actually speak', async () => {
	const screen = render(LanguagePill, { locale: 'en', offered: both });

	await screen.getByRole('button', { name: /Language/ }).click();

	await expect.element(screen.getByRole('menuitem', { name: 'English' })).toBeInTheDocument();
	await expect
		.element(screen.getByRole('menuitem', { name: 'Bahasa Indonesia' }))
		.toBeInTheDocument();
});

test('marks the one in use, so the menu says where you are', async () => {
	const screen = render(LanguagePill, { locale: 'id', offered: both });

	await screen.getByRole('button', { name: /Language/ }).click();

	await expect
		.element(screen.getByRole('menuitem', { name: 'Bahasa Indonesia' }))
		.toHaveAttribute('aria-current', 'true');
});

test('choosing one hands it over, and closes', async () => {
	const chosen: string[] = [];
	const screen = render(LanguagePill, {
		locale: 'en',
		offered: both,
		onchoose: (l: string) => chosen.push(l)
	});

	await screen.getByRole('button', { name: /Language/ }).click();
	await screen.getByRole('menuitem', { name: 'Bahasa Indonesia' }).click();

	expect(chosen).toEqual(['id']);
	await vi.waitFor(() =>
		expect(screen.getByRole('menuitem', { name: 'English' }).elements()).toHaveLength(0)
	);
});

test('has no accessibility violations, open or closed', async () => {
	const screen = render(LanguagePill, { locale: 'en', offered: both });

	await expectNoA11yViolations(document.body, UNSTYLED_GEOMETRY);

	await screen.getByRole('button', { name: /Language/ }).click();

	await expectNoA11yViolations(document.body, UNSTYLED_GEOMETRY);
});
