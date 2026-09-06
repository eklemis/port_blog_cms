import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { expectNoA11yViolations } from '$lib/shared/test/a11y';
import VerifyPage from './verify-page.svelte';

/**
 * The hold screen. It is where every new account starts, and its whole job is
 * to be worth looking at while nothing is happening: what verification unlocks,
 * the address the link went to, and a resend.
 *
 * Design: Screen / Verification gate 11:238 light · 11:252 dark ·
 * Tablet 131:4187 · Mobile 89:2285.
 */

// See sign-in-form.svelte.spec.ts: the harness mounts without a stylesheet, so
// axe measures unstyled line boxes rather than real pointer targets.
const UNSTYLED_GEOMETRY = { rules: { 'target-size': { enabled: false } } };

const props = { email: 'jane@example.com' };

beforeEach(() => vi.stubGlobal('fetch', vi.fn()));
afterEach(() => vi.unstubAllGlobals());

test('says what is being waited for', async () => {
	const screen = render(VerifyPage, props);

	await expect
		.element(screen.getByRole('heading', { level: 1 }))
		.toHaveTextContent('Verify your email to start publishing');
});

test('names the address the link went to', async () => {
	// Without it the screen cannot be acted on: someone who mistyped their
	// address has no way to find that out.
	const screen = render(VerifyPage, props);

	await expect.element(screen.getByText(/jane@example\.com/)).toBeInTheDocument();
});

test('says how long the link is good for', async () => {
	const screen = render(VerifyPage, props);

	await expect.element(screen.getByText(/24 hours/)).toBeInTheDocument();
});

test('says what is locked, so the state is legible rather than mysterious', async () => {
	const screen = render(VerifyPage, props);

	await expect
		.element(
			screen.getByText(
				'Until you open it you can browse your account, but posts, projects, résumés and uploads stay locked.'
			)
		)
		.toBeInTheDocument();
});

test('shows the account state as a word, not only as a colour', async () => {
	const screen = render(VerifyPage, props);

	await expect.element(screen.getByText('Awaiting verification')).toBeInTheDocument();
});

test('offers the resend', async () => {
	const screen = render(VerifyPage, props);

	await expect.element(screen.getByRole('button', { name: 'Resend the link' })).toBeInTheDocument();
});

test('has no amber button, because nothing here moves you forward', async () => {
	// The gate clears when the emailed link is opened. A primary action would
	// promise the app can do it for you.
	const screen = render(VerifyPage, props);

	for (const control of screen.getByRole('button').elements()) {
		expect(control.className).not.toMatch(/bg-arch-accent/);
	}
});

test('has no accessibility violations', async () => {
	render(VerifyPage, props);

	await expectNoA11yViolations(document.body, UNSTYLED_GEOMETRY);
});
