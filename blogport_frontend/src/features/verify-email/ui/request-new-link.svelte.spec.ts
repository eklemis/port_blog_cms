import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { expectNoA11yViolations } from '$lib/shared/test/a11y';
import RequestNewLink from './request-new-link.svelte';

/**
 * Asking for a fresh verification link when the old one is past using.
 *
 * It asks for the address rather than reading a session, because a verification
 * link is opened from mail — often on a different device than the account was
 * made on. The confirmation is deliberately neutral: saying that a particular
 * address was worth sending to would say whether it has an account.
 */

const NEUTRAL =
	'If that address is waiting to be verified, a new link is on its way. It is good for 24 hours.';

function stubFetch(status: number, body: unknown, headers: Record<string, string> = {}) {
	const fetchFn = vi.fn<typeof fetch>(
		async () => new Response(JSON.stringify(body), { status, headers })
	);
	vi.stubGlobal('fetch', fetchFn);
	return fetchFn;
}

const send = { name: 'Send a new link' };

beforeEach(() => vi.stubGlobal('fetch', vi.fn()));
afterEach(() => vi.unstubAllGlobals());

test('asks for the address, because there is no session to read one from', async () => {
	const screen = render(RequestNewLink, { onsent: () => {} });

	await expect.element(screen.getByLabelText('Email')).toBeInTheDocument();
	await expect.element(screen.getByRole('button', send)).toHaveAttribute('type', 'submit');
});

test('sends the typed address', async () => {
	const fetchFn = stubFetch(202, {});
	const screen = render(RequestNewLink, { onsent: () => {} });

	await screen.getByLabelText('Email').fill('jane@example.com');
	await screen.getByRole('button', send).click();

	await vi.waitFor(() => expect(fetchFn).toHaveBeenCalledOnce());
	const [, init] = fetchFn.mock.calls[0];
	expect(JSON.parse(init?.body as string)).toEqual({ email: 'jane@example.com' });
});

test('confirms without saying whether that address has an account', async () => {
	stubFetch(202, {});
	let confirmed: string | undefined;
	const screen = render(RequestNewLink, { onsent: (m: string) => (confirmed = m) });

	await screen.getByLabelText('Email').fill('nobody@example.com');
	await screen.getByRole('button', send).click();

	await vi.waitFor(() => expect(confirmed).toBe(NEUTRAL));
});

test('a half-typed address is caught before a round trip', async () => {
	const fetchFn = stubFetch(202, {});
	const screen = render(RequestNewLink, { onsent: () => {} });

	await screen.getByLabelText('Email').fill('jane@');
	await screen.getByRole('button', send).click();

	await expect
		.element(screen.getByText('That doesn’t look like an email address.'))
		.toBeInTheDocument();
	expect(fetchFn).not.toHaveBeenCalled();
});

test('an untouched field is not an error until submit', async () => {
	const screen = render(RequestNewLink, { onsent: () => {} });

	await screen.getByLabelText('Email').click();

	await expect.element(screen.getByLabelText('Email')).not.toHaveAttribute('aria-invalid');
});

test('a rate limit holds the button shut and counts down', async () => {
	stubFetch(429, { error: { code: 'RATE_LIMITED' } }, { 'retry-after': '3600' });
	const screen = render(RequestNewLink, { onsent: () => {} });

	await screen.getByLabelText('Email').fill('jane@example.com');
	await screen.getByRole('button', send).click();

	await expect
		.element(screen.getByRole('status'))
		.toHaveTextContent('Too many attempts. Try again in 60 minutes.');
	await expect.element(screen.getByRole('button', send)).toBeDisabled();
});

test('a failure does not pretend a link was sent', async () => {
	stubFetch(500, { error: { code: 'INTERNAL_ERROR' } });
	let confirmed: string | undefined;
	const screen = render(RequestNewLink, { onsent: (m: string) => (confirmed = m) });

	await screen.getByLabelText('Email').fill('jane@example.com');
	await screen.getByRole('button', send).click();

	await expect
		.element(screen.getByRole('status'))
		.toHaveTextContent('Something went wrong on our side.');
	expect(confirmed).toBeUndefined();
});

test('reports progress on a slow send, but not on a fast one', async () => {
	// Observed against the running backend: it sends mail before it answers, so
	// this takes seconds. A control that sits dead with no explanation for that
	// long reads as a broken page.
	vi.stubGlobal(
		'fetch',
		vi.fn<typeof fetch>(() => new Promise<Response>(() => {}))
	);
	const screen = render(RequestNewLink, { onsent: () => {} });

	await screen.getByLabelText('Email').fill('jane@example.com');
	const button = screen.getByRole('button', send);
	await button.click();

	// Immediately: shut, but not yet claiming to be busy.
	await expect.element(button).toBeDisabled();
	expect(button.element().getAttribute('aria-busy')).toBe('false');

	// After the 400ms floor: busy, with its accessible name unchanged.
	await vi.waitFor(() => expect(button.element().getAttribute('aria-busy')).toBe('true'), {
		timeout: 3000
	});
	await expect.element(screen.getByRole('button', send)).toBeInTheDocument();
});

test('has no accessibility violations', async () => {
	render(RequestNewLink, { onsent: () => {} });

	await expectNoA11yViolations(document.body, { rules: { 'target-size': { enabled: false } } });
});
