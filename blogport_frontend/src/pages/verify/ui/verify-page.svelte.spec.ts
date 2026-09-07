import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { expectNoA11yViolations } from '$lib/shared/test/a11y';
import { PRODUCT_NAME } from '$lib/shared/config/product';
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

test('carries the wordmark, and it goes home', async () => {
	// All three frames now draw the header; it used to be on mobile only.
	const screen = render(VerifyPage, props);

	await expect
		.element(screen.getByRole('link', { name: PRODUCT_NAME }))
		.toHaveAttribute('href', '/');
});

test('offers a way off the screen that is not the resend', async () => {
	// The escape the blueprint asks for. It was "Use a different address" in the
	// first cut, which no endpoint could back; signing out is a real action.
	const screen = render(VerifyPage, props);

	await expect.element(screen.getByRole('button', { name: 'Sign out' })).toBeInTheDocument();
});

test('signing out reports upward so the route can navigate', async () => {
	vi.stubGlobal(
		'fetch',
		vi.fn<typeof fetch>(async () => new Response('{}'))
	);
	let out = 0;
	const screen = render(VerifyPage, { ...props, onsignedout: () => out++ });

	await screen.getByRole('button', { name: 'Sign out' }).click();

	await vi.waitFor(() => expect(out).toBe(1));
});

test('announces what the resend reported, below both controls', async () => {
	const accepted = 'If that address needs verifying, a new link is on its way.';
	vi.stubGlobal(
		'fetch',
		vi.fn<typeof fetch>(
			async () => new Response(JSON.stringify({ message: accepted }), { status: 202 })
		)
	);
	const screen = render(VerifyPage, props);

	await screen.getByRole('button', { name: 'Resend the link' }).click();

	await expect.element(screen.getByRole('status')).toHaveTextContent(accepted);
});

test('has no accessibility violations', async () => {
	render(VerifyPage, props);

	await expectNoA11yViolations(document.body, UNSTYLED_GEOMETRY);
});
