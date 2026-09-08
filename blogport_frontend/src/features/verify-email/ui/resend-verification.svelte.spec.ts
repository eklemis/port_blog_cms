import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { expectNoA11yViolations } from '$lib/shared/test/a11y';
import ResendVerification from './resend-verification.svelte';

/**
 * The one control the hold screen offers. Not an amber button: nothing on this
 * screen moves the person forward, because only the emailed link can.
 */

const SENT = 'Sent. Check your inbox — the new link is good for 24 hours.';

function stubFetch(status: number, body: unknown, headers: Record<string, string> = {}) {
	const fetchFn = vi.fn<typeof fetch>(
		async () => new Response(JSON.stringify(body), { status, headers })
	);
	vi.stubGlobal('fetch', fetchFn);
	return fetchFn;
}

const button = () => ({ name: 'Resend the link' });

beforeEach(() => vi.stubGlobal('fetch', vi.fn()));
afterEach(() => vi.unstubAllGlobals());

test('offers to send the link again', async () => {
	const screen = render(ResendVerification, {});

	await expect.element(screen.getByRole('button', button())).toBeInTheDocument();
});

test('is not the primary action — nothing here is', async () => {
	// One amber button per screen, and this screen has none: the gate clears
	// when the emailed link is opened, not when a button is pressed.
	const screen = render(ResendVerification, {});

	await expect.element(screen.getByRole('button', button())).not.toHaveClass(/bg-arch-accent/);
});

test('asks the proxy when pressed', async () => {
	const fetchFn = stubFetch(202, {});
	const screen = render(ResendVerification, {});

	await screen.getByRole('button', button()).click();

	await vi.waitFor(() => expect(fetchFn).toHaveBeenCalledOnce());
	expect(fetchFn.mock.calls[0][0]).toBe('/api/auth/email-verification/resend');
});

test('reports the copy table’s sentence, not the backend’s', async () => {
	stubFetch(202, {});
	const said: (string | undefined)[] = [];
	const screen = render(ResendVerification, { onmessage: (m: string | undefined) => said.push(m) });

	await screen.getByRole('button', button()).click();

	await vi.waitFor(() => expect(said).toContain(SENT));
});

test('a second press cannot send two links', async () => {
	let release: (value: Response) => void = () => {};
	vi.stubGlobal(
		'fetch',
		vi.fn<typeof fetch>(() => new Promise<Response>((resolve) => (release = resolve)))
	);
	const screen = render(ResendVerification, {});

	const control = screen.getByRole('button', button());
	await control.click();

	await expect.element(control).toBeDisabled();

	release(new Response('{}', { status: 202 }));
});

test('stays pressable after a successful send — five an hour are allowed', async () => {
	stubFetch(202, {});
	const said: (string | undefined)[] = [];
	const screen = render(ResendVerification, { onmessage: (m: string | undefined) => said.push(m) });

	const control = screen.getByRole('button', button());
	await control.click();

	await vi.waitFor(() => expect(said).toContain(SENT));
	await expect.element(control).not.toBeDisabled();
});

test('a rate limit holds the button shut and explains why', async () => {
	stubFetch(429, { error: { code: 'RATE_LIMITED' } }, { 'retry-after': '3600' });
	const said: (string | undefined)[] = [];
	const screen = render(ResendVerification, { onmessage: (m: string | undefined) => said.push(m) });

	const control = screen.getByRole('button', button());
	await control.click();

	await vi.waitFor(() => expect(said).toContain('Too many attempts. Try again in 60 minutes.'));
	// A dead button with no explanation reads as a broken product at exactly the
	// moment someone is already annoyed.
	await expect.element(control).toBeDisabled();
	await expect
		.element(control)
		.toHaveAccessibleDescription('Too many attempts. Try again in 60 minutes.');
});

test('a lost session is handed upwards rather than shown as a failure', async () => {
	stubFetch(401, { error: { code: 'MISSING_AUTH_HEADER' } });
	let expired = 0;
	const screen = render(ResendVerification, { onsessionexpired: () => expired++ });

	await screen.getByRole('button', button()).click();

	await vi.waitFor(() => expect(expired).toBe(1));
});

test('has no accessibility violations at rest', async () => {
	render(ResendVerification, {});

	await expectNoA11yViolations();
});

test('has no accessibility violations while locked out', async () => {
	stubFetch(429, { error: { code: 'RATE_LIMITED' } }, { 'retry-after': '3600' });
	const screen = render(ResendVerification, {});

	await screen.getByRole('button', button()).click();
	await expect.element(screen.getByRole('button', button())).toBeDisabled();

	await expectNoA11yViolations();
});
