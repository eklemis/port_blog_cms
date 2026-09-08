import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { expectNoA11yViolations } from '$lib/shared/test/a11y';
import SetPasswordForm from './set-password-form.svelte';

const set = { name: 'Set password' };

/** Twelve or more characters, named so it reads as a fixture. */
const LONG_ENOUGH = 'fixture-12-chars-min';

function stubFetch(status: number, body: unknown, headers: Record<string, string> = {}) {
	const fetchFn = vi.fn<typeof fetch>(
		async () => new Response(JSON.stringify(body), { status, headers })
	);
	vi.stubGlobal('fetch', fetchFn);
	return fetchFn;
}

const props = { token: 'the-emailed-token', onset: () => {}, onexpired: () => {} };

beforeEach(() => vi.stubGlobal('fetch', vi.fn()));
afterEach(() => vi.unstubAllGlobals());

test('asks for one password, with a reveal rather than a second field', async () => {
	const screen = render(SetPasswordForm, props);

	await expect.element(screen.getByLabelText('New password')).toBeInTheDocument();
	await expect.element(screen.getByRole('button', { name: 'Show' })).toBeInTheDocument();
	// No confirm twin: revealing catches the typo it existed to catch.
	expect(screen.getByLabelText('Confirm password').elements()).toHaveLength(0);
});

test('states the rule from the start', async () => {
	const screen = render(SetPasswordForm, props);

	await expect.element(screen.getByText(/At least 12 characters/)).toBeInTheDocument();
});

test('sends the token from the URL with the new password', async () => {
	const fetchFn = stubFetch(200, { ok: true });
	let done = 0;
	const screen = render(SetPasswordForm, { ...props, onset: () => done++ });

	await screen.getByLabelText('New password').fill(LONG_ENOUGH);
	await screen.getByRole('button', set).click();

	await vi.waitFor(() => expect(done).toBe(1));
	expect(fetchFn.mock.calls[0][0]).toBe('/api/auth/password-reset/the-emailed-token');
});

test('a short password is caught before the token is spent', async () => {
	const fetchFn = stubFetch(200, { ok: true });
	const screen = render(SetPasswordForm, props);

	await screen.getByLabelText('New password').fill('short');
	await screen.getByRole('button', set).click();

	await expect.element(screen.getByText('At least 12 characters.')).toBeInTheDocument();
	expect(fetchFn).not.toHaveBeenCalled();
});

test('a refused password keeps the field and the token', async () => {
	// Losing a valid token to one weak password is a needless restart.
	stubFetch(400, { error: { code: 'INVALID_PASSWORD', message: 'Too weak for this server' } });
	let expired = 0;
	const screen = render(SetPasswordForm, { ...props, onexpired: () => expired++ });

	await screen.getByLabelText('New password').fill(LONG_ENOUGH);
	await screen.getByRole('button', set).click();

	await expect.element(screen.getByRole('status')).toHaveTextContent('Too weak for this server');
	await expect.element(screen.getByLabelText('New password')).toHaveValue(LONG_ENOUGH);
	expect(expired).toBe(0);
});

test('a stale link is handed upwards so the screen can offer a fresh one', async () => {
	stubFetch(401, { error: { code: 'INVALID_RESET_TOKEN' } });
	let expired = 0;
	const screen = render(SetPasswordForm, { ...props, onexpired: () => expired++ });

	await screen.getByLabelText('New password').fill(LONG_ENOUGH);
	await screen.getByRole('button', set).click();

	await vi.waitFor(() => expect(expired).toBe(1));
});

test('has no accessibility violations', async () => {
	render(SetPasswordForm, props);

	await expectNoA11yViolations(document.body, { rules: { 'target-size': { enabled: false } } });
});
