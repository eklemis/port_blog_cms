import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { expectNoA11yViolations } from '$lib/shared/test/a11y';
import RequestResetForm from './request-reset-form.svelte';

const send = { name: 'Send reset link' };

function stubFetch(status: number, body: unknown, headers: Record<string, string> = {}) {
	const fetchFn = vi.fn<typeof fetch>(
		async () => new Response(JSON.stringify(body), { status, headers })
	);
	vi.stubGlobal('fetch', fetchFn);
	return fetchFn;
}

beforeEach(() => vi.stubGlobal('fetch', vi.fn()));
afterEach(() => vi.unstubAllGlobals());

test('asks for one address and offers one action', async () => {
	const screen = render(RequestResetForm, { onsent: () => {} });

	await expect.element(screen.getByLabelText('Email')).toBeInTheDocument();
	await expect.element(screen.getByRole('button', send)).toHaveAttribute('type', 'submit');
});

test('reports the address it sent for, so the screen can confirm neutrally', async () => {
	stubFetch(200, { ok: true });
	let sentFor: string | undefined;
	const screen = render(RequestResetForm, { onsent: (e: string) => (sentFor = e) });

	await screen.getByLabelText('Email').fill('nobody@example.com');
	await screen.getByRole('button', send).click();

	await vi.waitFor(() => expect(sentFor).toBe('nobody@example.com'));
});

test('a half-typed address is caught before a round trip', async () => {
	const fetchFn = stubFetch(200, { ok: true });
	const screen = render(RequestResetForm, { onsent: () => {} });

	await screen.getByLabelText('Email').fill('jane@');
	await screen.getByRole('button', send).click();

	await expect
		.element(screen.getByText('That doesn’t look like an email address.'))
		.toBeInTheDocument();
	expect(fetchFn).not.toHaveBeenCalled();
});

test('an untouched field is not an error until submit', async () => {
	const screen = render(RequestResetForm, { onsent: () => {} });

	await screen.getByLabelText('Email').click();

	await expect.element(screen.getByLabelText('Email')).not.toHaveAttribute('aria-invalid');
});

test('a rate limit holds the button shut and counts down', async () => {
	stubFetch(429, { error: { code: 'RATE_LIMITED' } }, { 'retry-after': '3600' });
	const screen = render(RequestResetForm, { onsent: () => {} });

	await screen.getByLabelText('Email').fill('jane@example.com');
	await screen.getByRole('button', send).click();

	await expect
		.element(screen.getByRole('status'))
		.toHaveTextContent('Too many attempts. Try again in 60 minutes.');
	await expect.element(screen.getByRole('button', send)).toBeDisabled();
});

test('a failure does not pretend a link was sent', async () => {
	stubFetch(500, { error: { code: 'INTERNAL_ERROR' } });
	let sentFor: string | undefined;
	const screen = render(RequestResetForm, { onsent: (e: string) => (sentFor = e) });

	await screen.getByLabelText('Email').fill('jane@example.com');
	await screen.getByRole('button', send).click();

	await expect
		.element(screen.getByRole('status'))
		.toHaveTextContent('Something went wrong on our side.');
	expect(sentFor).toBeUndefined();
});

test('has no accessibility violations', async () => {
	render(RequestResetForm, { onsent: () => {} });

	await expectNoA11yViolations(document.body, { rules: { 'target-size': { enabled: false } } });
});
