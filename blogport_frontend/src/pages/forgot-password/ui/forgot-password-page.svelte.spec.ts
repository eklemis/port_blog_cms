import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { expectNoA11yViolations } from '$lib/shared/test/a11y';
import { PRODUCT_NAME } from '$lib/shared/config/product';
import ForgotPasswordPage from './forgot-password-page.svelte';

/**
 * Ask for a reset link, then the confirmation.
 *
 * The confirmation is the whole of J3: it must read the same whether or not
 * the address has an account, because branching would turn the form into a way
 * of discovering who is registered.
 */

const UNSTYLED_GEOMETRY = { rules: { 'target-size': { enabled: false } } };

function stubFetch(status: number, body: unknown) {
	vi.stubGlobal(
		'fetch',
		vi.fn<typeof fetch>(async () => new Response(JSON.stringify(body), { status }))
	);
}

async function ask(screen: ReturnType<typeof render>, email: string) {
	await screen.getByLabelText('Email').fill(email);
	await screen.getByRole('button', { name: 'Send reset link' }).click();
}

beforeEach(() => vi.stubGlobal('fetch', vi.fn()));
afterEach(() => vi.unstubAllGlobals());

test('carries the wordmark, and it goes home', async () => {
	const screen = render(ForgotPasswordPage, {});

	await expect
		.element(screen.getByRole('link', { name: PRODUCT_NAME }))
		.toHaveAttribute('href', '/');
});

test('opens on the form, not the confirmation', async () => {
	const screen = render(ForgotPasswordPage, {});

	await expect.element(screen.getByLabelText('Email')).toBeInTheDocument();
	expect(screen.getByText('Check your email').elements()).toHaveLength(0);
});

test('offers the way back without asking for anything', async () => {
	const screen = render(ForgotPasswordPage, {});

	await expect
		.element(screen.getByRole('link', { name: 'Back to sign in' }))
		.toHaveAttribute('href', '/auth/login');
});

test('confirms without saying whether the address was found', async () => {
	stubFetch(200, { ok: true });
	const screen = render(ForgotPasswordPage, {});

	await ask(screen, 'nobody@example.com');

	await expect
		.element(screen.getByRole('heading', { level: 1 }))
		.toHaveTextContent('Check your email');
	await expect
		.element(screen.getByText(/If an account exists for nobody@example\.com/))
		.toBeInTheDocument();
});

test('the confirmation is the same words for an address that exists', async () => {
	// One form, two addresses, identical output but for the address echoed back
	// — which the person typed, so it reveals nothing.
	stubFetch(200, { ok: true });
	const screen = render(ForgotPasswordPage, {});

	await ask(screen, 'jane@example.com');

	const said = (await screen.getByRole('heading', { level: 1 }).element()).textContent;
	expect(said).toBe('Check your email');
	// Nothing anywhere claims the address was found.
	const body = (await screen.getByRole('main').element()).textContent ?? '';
	expect(body).not.toMatch(/we found|is registered|no account|does not exist/i);
});

test('"Send again" goes back to the form rather than firing blind', async () => {
	stubFetch(200, { ok: true });
	const screen = render(ForgotPasswordPage, {});

	await ask(screen, 'jane@example.com');
	await screen.getByRole('button', { name: 'Send again' }).click();

	await expect.element(screen.getByLabelText('Email')).toBeInTheDocument();
});

test('has no accessibility violations on the form', async () => {
	render(ForgotPasswordPage, {});

	await expectNoA11yViolations(document.body, UNSTYLED_GEOMETRY);
});

test('has none on the confirmation either', async () => {
	stubFetch(200, { ok: true });
	const screen = render(ForgotPasswordPage, {});

	await ask(screen, 'jane@example.com');
	await expect.element(screen.getByText('Check your email')).toBeInTheDocument();

	await expectNoA11yViolations(document.body, UNSTYLED_GEOMETRY);
});
