import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { expectNoA11yViolations } from '$lib/shared/test/a11y';
import SignOutButton from './sign-out-button.svelte';

const control = { name: 'Sign out' };

beforeEach(() =>
	vi.stubGlobal(
		'fetch',
		vi.fn<typeof fetch>(async () => new Response('{}'))
	)
);
afterEach(() => vi.unstubAllGlobals());

test('offers the way out', async () => {
	const screen = render(SignOutButton, {});

	await expect.element(screen.getByRole('button', control)).toBeInTheDocument();
});

test('ends the session, then reports so the caller can navigate', async () => {
	const fetchFn = vi.fn<typeof fetch>(async () => new Response('{}'));
	vi.stubGlobal('fetch', fetchFn);
	let out = 0;
	const screen = render(SignOutButton, { onsignedout: () => out++ });

	await screen.getByRole('button', control).click();

	await vi.waitFor(() => expect(out).toBe(1));
	expect(fetchFn.mock.calls[0][0]).toBe('/api/auth/logout');
});

test('reports even when the request failed', async () => {
	vi.stubGlobal(
		'fetch',
		vi.fn<typeof fetch>(async () => {
			throw new TypeError('Failed to fetch');
		})
	);
	let out = 0;
	const screen = render(SignOutButton, { onsignedout: () => out++ });

	await screen.getByRole('button', control).click();

	await vi.waitFor(() => expect(out).toBe(1));
});

test('a second press cannot fire twice', async () => {
	let release: (value: Response) => void = () => {};
	vi.stubGlobal(
		'fetch',
		vi.fn<typeof fetch>(() => new Promise<Response>((resolve) => (release = resolve)))
	);
	const screen = render(SignOutButton, {});

	const button = screen.getByRole('button', control);
	await button.click();

	await expect.element(button).toBeDisabled();

	release(new Response('{}'));
});

test('has no accessibility violations', async () => {
	render(SignOutButton, {});

	await expectNoA11yViolations();
});
