import { expect, test } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { expectNoA11yViolations } from '$lib/shared/test/a11y';
import { PRODUCT_NAME } from '$lib/shared/config/product';
import SignInPage from './sign-in-page.svelte';

/**
 * The screen around the form: the shell, the heading, and the two ways off it.
 * Design: Figma Screen / Sign in 67:2 · 67:29 · Tablet 131:5161 · Mobile 89:2082.
 */

// See the note in sign-in-form.svelte.spec.ts: the harness mounts without a
// stylesheet, so axe measures unstyled line boxes rather than real targets.
const UNSTYLED_GEOMETRY = { rules: { 'target-size': { enabled: false } } };

test('the screen is headed by what you came to do', async () => {
	const screen = render(SignInPage, { onsignedin: () => {} });

	// "Sign in" is the page heading; the wordmark is branding, not an h1.
	await expect.element(screen.getByRole('heading', { level: 1 })).toHaveTextContent('Sign in');
});

test('the wordmark is the configured product name, and it goes home', async () => {
	const screen = render(SignInPage, { onsignedin: () => {} });

	// Asserted through the constant, not against the string "Arch": renaming the
	// product is then one edit in shared/config/product, and this test fails if
	// the markup ever spells it out again.
	await expect
		.element(screen.getByRole('link', { name: PRODUCT_NAME }))
		.toHaveAttribute('href', '/');
});

test('offers the other door for someone who has no account yet', async () => {
	const screen = render(SignInPage, { onsignedin: () => {} });

	await expect
		.element(screen.getByRole('link', { name: 'Create an account' }))
		.toHaveAttribute('href', '/auth/register');
});

test('a skip link is the first focusable thing on the page', async () => {
	const screen = render(SignInPage, { onsignedin: () => {} });

	const skip = screen.getByRole('link', { name: 'Skip to content' });
	await expect.element(skip).toHaveAttribute('href', '#main');
	// First in the DOM, because tab order is DOM order — Accessibility Spec §05.
	expect(screen.getByRole('link').elements()[0]).toBe(skip.element());
});

test('says nothing about a session when none expired', async () => {
	const screen = render(SignInPage, { onsignedin: () => {} });

	expect(screen.getByText(/session expired/i).elements()).toHaveLength(0);
});

test('an expired session says why you are here, in the blueprint’s words', async () => {
	const screen = render(SignInPage, { sessionExpired: true, onsignedin: () => {} });

	await expect
		.element(screen.getByText('Your session expired. Sign in to pick up where you left off.'))
		.toBeInTheDocument();
});

test('has no accessibility violations', async () => {
	render(SignInPage, { onsignedin: () => {} });

	await expectNoA11yViolations(document.body, UNSTYLED_GEOMETRY);
});

test('has no accessibility violations while explaining an expired session', async () => {
	render(SignInPage, { sessionExpired: true, onsignedin: () => {} });

	await expectNoA11yViolations(document.body, UNSTYLED_GEOMETRY);
});
