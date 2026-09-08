import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { expectNoA11yViolations } from '$lib/shared/test/a11y';
import { PRODUCT_NAME } from '$lib/shared/config/product';
import RegisterPage from './register-page.svelte';

/**
 * The screen around the form. Design: Screen / Create account 67:56 ·
 * Mobile 89:2136 · Tablet 166:6278.
 */

const UNSTYLED_GEOMETRY = { rules: { 'target-size': { enabled: false } } };

beforeEach(() => vi.stubGlobal('fetch', vi.fn()));
afterEach(() => vi.unstubAllGlobals());

test('is headed by what you came to do', async () => {
	const screen = render(RegisterPage, { onregistered: () => {} });

	await expect
		.element(screen.getByRole('heading', { level: 1 }))
		.toHaveTextContent('Create your account');
});

test('carries the wordmark, and it goes home', async () => {
	const screen = render(RegisterPage, { onregistered: () => {} });

	await expect
		.element(screen.getByRole('link', { name: PRODUCT_NAME }))
		.toHaveAttribute('href', '/');
});

test('a skip link is the first focusable thing on the page', async () => {
	const screen = render(RegisterPage, { onregistered: () => {} });

	const skip = screen.getByRole('link', { name: 'Skip to content' });
	await expect.element(skip).toHaveAttribute('href', '#main');
	expect(screen.getByRole('link').elements()[0]).toBe(skip.element());
});

test('holds the form', async () => {
	const screen = render(RegisterPage, { onregistered: () => {} });

	await expect.element(screen.getByLabelText('Username')).toBeInTheDocument();
	await expect.element(screen.getByRole('button', { name: 'Create account' })).toBeInTheDocument();
});

test('has no accessibility violations', async () => {
	render(RegisterPage, { onregistered: () => {} });

	await expectNoA11yViolations(document.body, UNSTYLED_GEOMETRY);
});
