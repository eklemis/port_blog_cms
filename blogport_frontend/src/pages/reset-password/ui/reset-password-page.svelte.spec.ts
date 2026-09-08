import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { expectNoA11yViolations } from '$lib/shared/test/a11y';
import ResetPasswordPage from './reset-password-page.svelte';

/**
 * Set a new password from the emailed link.
 *
 * Note what this screen cannot say: which address it belongs to. There is no
 * GET on a reset token and the success body carries only a message, so the
 * frame's "For jane@example.com." has nothing behind it. See the PR.
 */

const UNSTYLED_GEOMETRY = { rules: { 'target-size': { enabled: false } } };
const LONG_ENOUGH = 'fixture-12-chars-min';

function stubFetch(status: number, body: unknown) {
	vi.stubGlobal(
		'fetch',
		vi.fn<typeof fetch>(async () => new Response(JSON.stringify(body), { status }))
	);
}

async function setIt(screen: ReturnType<typeof render>) {
	await screen.getByLabelText('New password').fill(LONG_ENOUGH);
	await screen.getByRole('button', { name: 'Set password' }).click();
}

const props = { token: 'the-emailed-token' };

beforeEach(() => vi.stubGlobal('fetch', vi.fn()));
afterEach(() => vi.unstubAllGlobals());

test('opens on the form', async () => {
	const screen = render(ResetPasswordPage, props);

	await expect
		.element(screen.getByRole('heading', { level: 1 }))
		.toHaveTextContent('Set a new password');
	await expect.element(screen.getByLabelText('New password')).toBeInTheDocument();
});

test('says the link can be replaced before it is found to be stale', async () => {
	const screen = render(ResetPasswordPage, props);

	await expect.element(screen.getByText(/If this link has expired/)).toBeInTheDocument();
});

test('confirms the change, and that other sessions went with it', async () => {
	// The revocation is the API's own behaviour, and someone resetting a
	// password often does it because they think somebody else had it.
	stubFetch(200, { ok: true });
	const screen = render(ResetPasswordPage, props);

	await setIt(screen);

	await expect
		.element(screen.getByRole('heading', { level: 1 }))
		.toHaveTextContent('Password changed.');
	await expect.element(screen.getByText(/Every other session was signed out/)).toBeInTheDocument();
});

test('sends them to sign in afterwards, because the reset returns no session', async () => {
	stubFetch(200, { ok: true });
	const screen = render(ResetPasswordPage, props);

	await setIt(screen);

	await expect
		.element(screen.getByRole('link', { name: 'Sign in' }))
		.toHaveAttribute('href', '/auth/login');
});

test('a stale link offers a fresh one from this same screen', async () => {
	// The dead end to avoid: never bounce back to login.
	stubFetch(401, { error: { code: 'INVALID_RESET_TOKEN' } });
	const screen = render(ResetPasswordPage, props);

	await setIt(screen);

	await expect
		.element(screen.getByRole('heading', { level: 1 }))
		.toHaveTextContent('This reset link is no longer valid.');
	await expect
		.element(screen.getByRole('link', { name: 'Send a new link' }))
		.toHaveAttribute('href', '/auth/forgot');
});

test('a refused password leaves the form standing', async () => {
	stubFetch(400, { error: { code: 'INVALID_PASSWORD', message: 'Too weak for this server' } });
	const screen = render(ResetPasswordPage, props);

	await setIt(screen);

	await expect.element(screen.getByRole('status')).toHaveTextContent('Too weak for this server');
	await expect.element(screen.getByLabelText('New password')).toBeInTheDocument();
});

test('has no accessibility violations on the form', async () => {
	render(ResetPasswordPage, props);

	await expectNoA11yViolations(document.body, UNSTYLED_GEOMETRY);
});

test('has none once the password is changed', async () => {
	stubFetch(200, { ok: true });
	const screen = render(ResetPasswordPage, props);

	await setIt(screen);
	await expect.element(screen.getByText('Password changed.')).toBeInTheDocument();

	await expectNoA11yViolations(document.body, UNSTYLED_GEOMETRY);
});
