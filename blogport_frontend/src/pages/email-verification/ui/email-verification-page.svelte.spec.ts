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

const verified = { verified: true as const, hasSession: false };
const dead = {
	verified: false as const,
	message: 'This link has expired.',
	dead: true,
	hasSession: false
};

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

	await expect
		.element(screen.getByRole('heading', { level: 1 }))
		.toHaveTextContent("You're verified.");
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
	const screen = render(EmailVerificationPage, { ...verified, hasSession: true });

	await expect
		.element(screen.getByRole('link', { name: 'Go to your studio' }))
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

test('offers a new link on the same screen, with no session needed', async () => {
	// The dead end to avoid. Never bounce back to login — and the endpoint takes
	// an address, so the screen can simply ask for one.
	const screen = render(EmailVerificationPage, dead);

	await expect.element(screen.getByLabelText('Email')).toBeInTheDocument();
	await expect.element(screen.getByRole('button', { name: 'Send a new link' })).toBeInTheDocument();
});

test('confirms neutrally once a new link is asked for', async () => {
	vi.stubGlobal(
		'fetch',
		vi.fn<typeof fetch>(async () => new Response('{}', { status: 202 }))
	);
	const screen = render(EmailVerificationPage, dead);

	await screen.getByLabelText('Email').fill('nobody@example.com');
	await screen.getByRole('button', { name: 'Send a new link' }).click();

	await expect
		.element(screen.getByRole('heading', { level: 1 }))
		.toHaveTextContent('Sent, if it was needed.');
	// Nothing here says whether that address has an account.
	await expect
		.element(screen.getByText(/If that address is waiting to be verified/))
		.toBeInTheDocument();
});

test('a fault on our side is not blamed on the link', async () => {
	const screen = render(EmailVerificationPage, {
		verified: false as const,
		message: 'Something went wrong on our side.',
		dead: false,
		hasSession: false
	});

	await expect
		.element(screen.getByRole('heading', { level: 1 }))
		.toHaveTextContent('Something went wrong on our side.');
	// No form: asking for another link when the server is broken fails the same way.
	expect(screen.getByRole('button', { name: 'Send a new link' }).elements()).toHaveLength(0);
});

// ── accessibility ──────────────────────────────────────────────────────────

test('the verified screen has no accessibility violations', async () => {
	render(EmailVerificationPage, verified);

	await expectNoA11yViolations(document.body, UNSTYLED_GEOMETRY);
});

test('the stale-link screen has no accessibility violations', async () => {
	render(EmailVerificationPage, dead);

	await expectNoA11yViolations(document.body, UNSTYLED_GEOMETRY);
});
