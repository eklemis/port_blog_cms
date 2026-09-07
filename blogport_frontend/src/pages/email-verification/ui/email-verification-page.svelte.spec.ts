import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { expectNoA11yViolations } from '$lib/shared/test/a11y';
import { PRODUCT_NAME } from '$lib/shared/config/product';
import EmailVerificationPage from './email-verification-page.svelte';

/**
 * What the emailed link shows when it lands. No frame exists for this screen —
 * the Console Blueprint specifies it in prose only — so it is built from the
 * verification gate's card, which is the family it belongs to.
 */

const UNSTYLED_GEOMETRY = { rules: { 'target-size': { enabled: false } } };

const verified = { verified: true as const, canResend: false };
const dead = { verified: false as const, message: 'This link has expired.', canResend: false };

beforeEach(() => vi.stubGlobal('fetch', vi.fn()));
afterEach(() => vi.unstubAllGlobals());

test('carries the wordmark, like the rest of the auth surface', async () => {
	const screen = render(EmailVerificationPage, verified);

	await expect
		.element(screen.getByRole('link', { name: PRODUCT_NAME }))
		.toHaveAttribute('href', '/');
});

// ── verified ───────────────────────────────────────────────────────────────

test('says so when the address is verified', async () => {
	const screen = render(EmailVerificationPage, verified);

	await expect.element(screen.getByRole('heading', { level: 1 })).toHaveTextContent(/verified/i);
});

test('a signed-out visitor is pointed at sign in', async () => {
	const screen = render(EmailVerificationPage, verified);

	await expect
		.element(screen.getByRole('link', { name: 'Sign in' }))
		.toHaveAttribute('href', '/auth/login');
});

test('someone who was already signed in is pointed at the console', async () => {
	// Their session still says unverified — it was minted before this — but the
	// next request rebuilds it from the profile endpoint, which reads the row.
	const screen = render(EmailVerificationPage, { ...verified, canResend: true });

	await expect
		.element(screen.getByRole('link', { name: 'Go to your console' }))
		.toHaveAttribute('href', '/studio');
});

test('never scolds someone for opening a link twice', async () => {
	const screen = render(EmailVerificationPage, verified);

	const body = (await screen.getByRole('main').element()).textContent ?? '';
	expect(body).not.toMatch(/already|again|expired|invalid|sorry/i);
});

// ── the link is past using ─────────────────────────────────────────────────

test('leads with the blueprint’s sentence when the link is stale', async () => {
	const screen = render(EmailVerificationPage, dead);

	await expect
		.element(screen.getByRole('heading', { level: 1 }))
		.toHaveTextContent('This link has expired.');
});

test('offers a new link on the same screen when it can', async () => {
	// The dead end to avoid. Never bounce back to login.
	const screen = render(EmailVerificationPage, { ...dead, canResend: true });

	await expect.element(screen.getByRole('button', { name: 'Resend the link' })).toBeInTheDocument();
});

test('does not offer a resend it cannot perform', async () => {
	// Resend reads the address from the session; with no session there is none,
	// and a button that always fails is worse than a sentence saying what to do.
	const screen = render(EmailVerificationPage, dead);

	expect(screen.getByRole('button', { name: 'Resend the link' }).elements()).toHaveLength(0);
	await expect.element(screen.getByRole('link', { name: 'Sign in' })).toBeInTheDocument();
});

test('a fault on our side is not blamed on the link', async () => {
	const screen = render(EmailVerificationPage, {
		verified: false as const,
		message: 'Something went wrong on our side.',
		canResend: false
	});

	await expect
		.element(screen.getByRole('heading', { level: 1 }))
		.toHaveTextContent('Something went wrong on our side.');
});

// ── accessibility ──────────────────────────────────────────────────────────

test('the verified screen has no accessibility violations', async () => {
	render(EmailVerificationPage, verified);

	await expectNoA11yViolations(document.body, UNSTYLED_GEOMETRY);
});

test('the stale-link screen has no accessibility violations', async () => {
	render(EmailVerificationPage, { ...dead, canResend: true });

	await expectNoA11yViolations(document.body, UNSTYLED_GEOMETRY);
});
