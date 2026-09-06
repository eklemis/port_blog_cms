import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { expectNoA11yViolations } from '$lib/shared/test/a11y';
import SignInForm from './sign-in-form.svelte';

/**
 * The sign-in form. Three behaviours here are the reason this screen is worth
 * a spec of its own rather than a glance:
 *
 *   · Verification, not login, is the gate — an unverified account signs in
 *     successfully and must still be sent to the hold screen.
 *   · A failed sign-in says one thing for a wrong address and a wrong password.
 *   · The password rule is length, 12–128, and nothing else.
 */

const VERIFIED = {
	email: 'jane@example.com',
	id: '123e4567-e89b-12d3-a456-426614174000',
	is_verified: true,
	username: 'janedoe'
};

const UNVERIFIED = { ...VERIFIED, is_verified: false };

const GOOD_PASSWORD = 'a-real-password';

function stubFetch(status: number, body: unknown, headers: Record<string, string> = {}) {
	const fetchFn = vi.fn<typeof fetch>(
		async () => new Response(JSON.stringify(body), { status, headers })
	);
	vi.stubGlobal('fetch', fetchFn);
	return fetchFn;
}

function signedIn(user = VERIFIED) {
	return stubFetch(200, { user });
}

function rejected() {
	return stubFetch(401, {
		error: { code: 'INVALID_CREDENTIALS', message: 'Invalid email or password' }
	});
}

/** Fill both fields and press the one amber button. */
async function signIn(
	screen: ReturnType<typeof render>,
	email = 'jane@example.com',
	password = GOOD_PASSWORD
) {
	await screen.getByLabelText('Email').fill(email);
	await screen.getByLabelText('Password').fill(password);
	await screen.getByRole('button', { name: 'Sign in' }).click();
}

beforeEach(() => {
	vi.stubGlobal('fetch', vi.fn());
});

afterEach(() => {
	vi.unstubAllGlobals();
});

// ── what is on the screen ──────────────────────────────────────────────────

test('asks for an email and a password, and nothing else', async () => {
	const screen = render(SignInForm, { onsignedin: () => {} });

	await expect.element(screen.getByLabelText('Email')).toBeInTheDocument();
	await expect.element(screen.getByLabelText('Password')).toBeInTheDocument();
});

test('offers the way out beside the password, where someone stuck is looking', async () => {
	const screen = render(SignInForm, { onsignedin: () => {} });

	await expect
		.element(screen.getByRole('link', { name: 'Forgot password?' }))
		.toHaveAttribute('href', '/auth/forgot');
});

test('the one primary action submits the form', async () => {
	const screen = render(SignInForm, { onsignedin: () => {} });

	await expect
		.element(screen.getByRole('button', { name: 'Sign in' }))
		.toHaveAttribute('type', 'submit');
});

// ── the happy path, and the value round trip ───────────────────────────────

test('posts what was typed to the proxy route', async () => {
	const fetchFn = signedIn();
	const screen = render(SignInForm, { onsignedin: () => {} });

	await signIn(screen);

	await vi.waitFor(() => expect(fetchFn).toHaveBeenCalledOnce());
	const [url, init] = fetchFn.mock.calls[0];
	expect(url).toBe('/api/auth/login');
	expect(JSON.parse(init?.body as string)).toEqual({
		email: 'jane@example.com',
		password: GOOD_PASSWORD
	});
});

// ── the gate ───────────────────────────────────────────────────────────────

test('a verified author is sent to the console', async () => {
	signedIn();
	let destination: string | undefined;
	const screen = render(SignInForm, { onsignedin: (to: string) => (destination = to) });

	await signIn(screen);

	await vi.waitFor(() => expect(destination).toBe('/studio'));
});

test('a verified author returns to where they were sent away from', async () => {
	signedIn();
	let destination: string | undefined;
	const screen = render(SignInForm, {
		next: '/studio/posts/new',
		onsignedin: (to: string) => (destination = to)
	});

	await signIn(screen);

	await vi.waitFor(() => expect(destination).toBe('/studio/posts/new'));
});

test('an unverified account signs in successfully and still goes to the hold screen', async () => {
	// The one that matters. Login returns 200 and a working token for an
	// unverified account; every authoring route then 403s with
	// EMAIL_NOT_VERIFIED. Sending them to /studio would render an authoring UI
	// that fails on every button. Console Blueprint §02.
	signedIn(UNVERIFIED);
	let destination: string | undefined;
	const screen = render(SignInForm, { onsignedin: (to: string) => (destination = to) });

	await signIn(screen);

	await vi.waitFor(() => expect(destination).toBe('/verify'));
});

test('a saved destination does not carry an unverified account into the console', async () => {
	signedIn(UNVERIFIED);
	let destination: string | undefined;
	const screen = render(SignInForm, {
		next: '/studio/posts/new',
		onsignedin: (to: string) => (destination = to)
	});

	await signIn(screen);

	await vi.waitFor(() => expect(destination).toBe('/verify'));
});

// ── the deliberate ambiguity ───────────────────────────────────────────────

test('a failed sign-in says the one sentence, and stays put', async () => {
	rejected();
	let destination: string | undefined;
	const screen = render(SignInForm, { onsignedin: (to: string) => (destination = to) });

	await signIn(screen, 'jane@example.com', 'wrong-password-x');

	await expect
		.element(screen.getByText("That email and password don't match."))
		.toBeInTheDocument();
	expect(destination).toBeUndefined();
});

test('an unknown address and a wrong password read identically on screen', async () => {
	// Nothing rendered may distinguish the two — doing so would turn the sign-in
	// screen into a way of discovering which addresses have accounts. One form,
	// two attempts, so the two states are compared in the same DOM.
	rejected();
	const screen = render(SignInForm, { onsignedin: () => {} });

	async function stateAfter(email: string, password: string) {
		await signIn(screen, email, password);
		const message = screen.getByRole('status');
		await expect.element(message).toHaveTextContent("That email and password don't match.");
		return {
			message: message.element().textContent?.trim(),
			emailInvalid: screen.getByLabelText('Email').element().getAttribute('aria-invalid'),
			passwordInvalid: screen.getByLabelText('Password').element().getAttribute('aria-invalid')
		};
	}

	const unknownAddress = await stateAfter('nobody-at-all@example.com', 'a-real-password');
	const wrongPassword = await stateAfter('jane@example.com', 'wrong-password-x');

	expect(unknownAddress).toEqual(wrongPassword);
	// And neither points at a field: a marked email would say the address was
	// the half that was wrong.
	expect(unknownAddress.emailInvalid).toBeNull();
	expect(unknownAddress.passwordInvalid).toBeNull();
});

test('a rejected form keeps every value', async () => {
	rejected();
	const screen = render(SignInForm, { onsignedin: () => {} });

	await signIn(screen, 'jane@example.com', 'wrong-password-x');

	await expect
		.element(screen.getByText("That email and password don't match."))
		.toBeInTheDocument();
	// Losing a filled form to one bad field is the worst outcome available.
	await expect.element(screen.getByLabelText('Email')).toHaveValue('jane@example.com');
	await expect.element(screen.getByLabelText('Password')).toHaveValue('wrong-password-x');
});

// ── validation, and its timing ─────────────────────────────────────────────

test('a half-typed address is not an error yet', async () => {
	const screen = render(SignInForm, { onsignedin: () => {} });

	await screen.getByLabelText('Email').fill('j');

	// `j` is not an invalid email; it is the first letter of one.
	await expect.element(screen.getByLabelText('Email')).not.toHaveAttribute('aria-invalid');
});

test('the address is checked when the field is left', async () => {
	const screen = render(SignInForm, { onsignedin: () => {} });

	await screen.getByLabelText('Email').fill('jane@');
	await screen.getByLabelText('Password').click();

	await expect
		.element(screen.getByText('That doesn’t look like an email address.'))
		.toBeInTheDocument();
});

test('an untouched empty field is not an error until submit', async () => {
	const screen = render(SignInForm, { onsignedin: () => {} });

	// Focus it and leave without typing.
	await screen.getByLabelText('Email').click();
	await screen.getByLabelText('Password').click();

	await expect.element(screen.getByLabelText('Email')).not.toHaveAttribute('aria-invalid');
});

test('a short password is caught here rather than spending a round trip', async () => {
	const fetchFn = stubFetch(200, { user: VERIFIED });
	const screen = render(SignInForm, { onsignedin: () => {} });

	await signIn(screen, 'jane@example.com', 'short');

	await expect.element(screen.getByText('At least 12 characters.')).toBeInTheDocument();
	expect(fetchFn).not.toHaveBeenCalled();
});

test('twelve plain lowercase characters are accepted — there is no complexity rule', async () => {
	// The OpenAPI example is `SecurePass123!`, which implies uppercase, digit
	// and symbol rules the server does not have. Inventing them here would
	// reject passwords the backend would happily accept.
	const fetchFn = signedIn();
	const screen = render(SignInForm, { onsignedin: () => {} });

	await signIn(screen, 'jane@example.com', 'abcdefghijkl');

	await vi.waitFor(() => expect(fetchFn).toHaveBeenCalledOnce());
});

test('submit focuses the first field that failed', async () => {
	const screen = render(SignInForm, { onsignedin: () => {} });

	await screen.getByLabelText('Password').fill(GOOD_PASSWORD);
	await screen.getByRole('button', { name: 'Sign in' }).click();

	await vi.waitFor(() =>
		expect(document.activeElement).toBe(screen.getByLabelText('Email').element())
	);
});

test('an error clears on the next blur that passes', async () => {
	const screen = render(SignInForm, { onsignedin: () => {} });
	const email = screen.getByLabelText('Email');

	await email.fill('jane@');
	await screen.getByLabelText('Password').click();
	await expect.element(email).toHaveAttribute('aria-invalid', 'true');

	await email.fill('jane@example.com');
	await screen.getByLabelText('Password').click();

	// Not on focus, which would hide the message the moment someone came back
	// to read it — on the next blur that passes.
	await expect.element(email).not.toHaveAttribute('aria-invalid');
});

// ── in flight ──────────────────────────────────────────────────────────────

test('a second press cannot double-submit', async () => {
	let release: (value: Response) => void = () => {};
	const fetchFn = vi.fn<typeof fetch>(
		() => new Promise<Response>((resolve) => (release = resolve))
	);
	vi.stubGlobal('fetch', fetchFn);
	const screen = render(SignInForm, { onsignedin: () => {} });

	await screen.getByLabelText('Email').fill('jane@example.com');
	await screen.getByLabelText('Password').fill(GOOD_PASSWORD);
	const button = screen.getByRole('button', { name: 'Sign in' });
	await button.click();

	await expect.element(button).toBeDisabled();
	expect(fetchFn).toHaveBeenCalledOnce();

	release(new Response(JSON.stringify({ user: VERIFIED }), { status: 200 }));
});

// ── rate limiting ──────────────────────────────────────────────────────────

test('a rate limit counts down and holds the button shut', async () => {
	stubFetch(
		429,
		{ error: { code: 'RATE_LIMITED', message: 'Too many requests' } },
		{
			'retry-after': '300'
		}
	);
	const screen = render(SignInForm, { onsignedin: () => {} });

	await signIn(screen);

	// A dead button with no explanation reads as a broken product at exactly the
	// moment someone is already annoyed. Blueprint §07.
	await expect
		.element(screen.getByRole('status'))
		.toHaveTextContent('Too many attempts. Try again in 5 minutes.');
	const button = screen.getByRole('button', { name: 'Sign in' });
	await expect.element(button).toBeDisabled();
	// A dead button with no explanation reads as a broken product; the reason
	// travels with it.
	await expect
		.element(button)
		.toHaveAccessibleDescription('Too many attempts. Try again in 5 minutes.');
});

// ── accessibility ──────────────────────────────────────────────────────────

/**
 * `target-size` is off for these two, and only these two.
 *
 * The harness mounts a component with no stylesheet attached, so axe measures
 * unstyled text: every control collapses to its intrinsic line box and the
 * "Forgot password?" link is reported at about 14px beside the password input.
 * With the real stylesheet the link is a 24px-tall inline-flex target, which is
 * what `min-h-6` in the markup is for. Geometry is checked in the browser at
 * 390, 834 and 1180 instead — Accessibility Spec §07 and §15.
 */
const UNSTYLED_GEOMETRY = { rules: { 'target-size': { enabled: false } } };

test('the resting form has no accessibility violations', async () => {
	render(SignInForm, { onsignedin: () => {} });

	await expectNoA11yViolations(document.body, UNSTYLED_GEOMETRY);
});

test('the rejected form has no accessibility violations', async () => {
	rejected();
	const screen = render(SignInForm, { onsignedin: () => {} });

	await signIn(screen, 'jane@example.com', 'wrong-password-x');
	await expect
		.element(screen.getByText("That email and password don't match."))
		.toBeInTheDocument();

	await expectNoA11yViolations(document.body, UNSTYLED_GEOMETRY);
});
