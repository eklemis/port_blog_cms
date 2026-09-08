import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { expectNoA11yViolations } from '$lib/shared/test/a11y';
import RegisterForm from './register-form.svelte';

/**
 * Create your account. Four fields, one amber button.
 *
 * The branch worth the most here is the collision: someone who already has an
 * account almost never wants a second one, so 409 marks the email field and
 * offers the two ways out beside it rather than just refusing.
 */

const UNSTYLED_GEOMETRY = { rules: { 'target-size': { enabled: false } } };

/**
 * Long enough to clear the only rule, and named rather than written inline so
 * it reads as what it is: a fixture, not anybody's password.
 */
const LONG_ENOUGH = 'fixture-12-chars-min';

/**
 * Twelve lowercase letters and nothing else — no capital, no digit, no symbol.
 * That is the whole point of the test that uses it: the server has no
 * complexity rule, and this form must not invent one.
 */
const NO_COMPLEXITY = 'abcdefghijkl';

/** Below the only rule there is. */
const TOO_SHORT = 'short';

function stubFetch(status: number, body: unknown, headers: Record<string, string> = {}) {
	const fetchFn = vi.fn<typeof fetch>(
		async () => new Response(JSON.stringify(body), { status, headers })
	);
	vi.stubGlobal('fetch', fetchFn);
	return fetchFn;
}

const create = { name: 'Create account' };

async function fillIn(screen: ReturnType<typeof render>, over: Record<string, string> = {}) {
	const values: Record<string, string> = {
		Username: 'janedoe',
		Email: 'jane@example.com',
		'Full name': 'Jane Doe',
		Password: LONG_ENOUGH,
		...over
	};
	for (const [label, value] of Object.entries(values)) {
		await screen.getByLabelText(label).fill(value);
	}
}

beforeEach(() => vi.stubGlobal('fetch', vi.fn()));
afterEach(() => vi.unstubAllGlobals());

// ── the form ───────────────────────────────────────────────────────────────

test('asks for the four things register takes', async () => {
	const screen = render(RegisterForm, { onregistered: () => {} });

	for (const label of ['Username', 'Email', 'Full name', 'Password']) {
		await expect.element(screen.getByLabelText(label)).toBeInTheDocument();
	}
});

test('says the username is permanent before it is chosen, not after', async () => {
	const screen = render(RegisterForm, { onregistered: () => {} });

	await expect.element(screen.getByText(/Permanent/)).toBeInTheDocument();
});

test('shows the address the username will actually become', async () => {
	// Lowercased on the way in. Someone typing JaneDoe should see /janedoe now
	// rather than discover it afterwards, because it never changes again.
	const screen = render(RegisterForm, { onregistered: () => {} });

	await screen.getByLabelText('Username').fill('JaneDoe');

	await expect.element(screen.getByText(/\/janedoe/)).toBeInTheDocument();
});

test('states the password rule from the start', async () => {
	const screen = render(RegisterForm, { onregistered: () => {} });

	await expect
		.element(screen.getByText('At least 12 characters. That is the only rule.'))
		.toBeInTheDocument();
});

// ── success ────────────────────────────────────────────────────────────────

test('sends what was typed and reports success', async () => {
	const fetchFn = stubFetch(201, { user: { username: 'janedoe' } });
	let registered = 0;
	const screen = render(RegisterForm, { onregistered: () => registered++ });

	await fillIn(screen);
	await screen.getByRole('button', create).click();

	await vi.waitFor(() => expect(registered).toBe(1));
	const [, init] = fetchFn.mock.calls[0];
	expect(JSON.parse(init?.body as string)).toEqual({
		username: 'janedoe',
		email: 'jane@example.com',
		full_name: 'Jane Doe',
		password: LONG_ENOUGH
	});
});

// ── the collision ──────────────────────────────────────────────────────────

test('an address already registered marks the email field', async () => {
	stubFetch(409, { error: { code: 'USER_ALREADY_EXISTS' } });
	const screen = render(RegisterForm, { onregistered: () => {} });

	await fillIn(screen);
	await screen.getByRole('button', create).click();

	await expect
		.element(screen.getByText("There's already an account for that address."))
		.toBeInTheDocument();
	await expect.element(screen.getByLabelText('Email')).toHaveAttribute('aria-invalid', 'true');
});

test('and offers the two ways out beside it', async () => {
	// The person almost certainly has an account they forgot.
	stubFetch(409, { error: { code: 'USER_ALREADY_EXISTS' } });
	const screen = render(RegisterForm, { onregistered: () => {} });

	await fillIn(screen);
	await screen.getByRole('button', create).click();

	await expect
		.element(screen.getByRole('link', { name: 'Sign in instead' }))
		.toHaveAttribute('href', '/auth/login');
	await expect
		.element(screen.getByRole('link', { name: 'Reset your password' }))
		.toHaveAttribute('href', '/auth/forgot');
});

test('the ways out are not there until they are needed', async () => {
	const screen = render(RegisterForm, { onregistered: () => {} });

	expect(screen.getByRole('link', { name: 'Sign in instead' }).elements()).toHaveLength(0);
});

test('a rejected form keeps every value', async () => {
	stubFetch(409, { error: { code: 'USER_ALREADY_EXISTS' } });
	const screen = render(RegisterForm, { onregistered: () => {} });

	await fillIn(screen);
	await screen.getByRole('button', create).click();

	await expect
		.element(screen.getByText("There's already an account for that address."))
		.toBeInTheDocument();
	await expect.element(screen.getByLabelText('Username')).toHaveValue('janedoe');
	await expect.element(screen.getByLabelText('Full name')).toHaveValue('Jane Doe');
	await expect.element(screen.getByLabelText('Password')).toHaveValue(LONG_ENOUGH);
});

// ── validation ─────────────────────────────────────────────────────────────

test('catches a bad username before a round trip', async () => {
	const fetchFn = stubFetch(201, {});
	const screen = render(RegisterForm, { onregistered: () => {} });

	await fillIn(screen, { Username: 'jane doe' });
	await screen.getByRole('button', create).click();

	await expect
		.element(screen.getByText('Letters, numbers and underscores only.'))
		.toBeInTheDocument();
	expect(fetchFn).not.toHaveBeenCalled();
});

test('catches a short password before a round trip, and adds no other rule', async () => {
	const fetchFn = stubFetch(201, {});
	const screen = render(RegisterForm, { onregistered: () => {} });

	await fillIn(screen, { Password: TOO_SHORT });
	await screen.getByRole('button', create).click();

	await expect.element(screen.getByText('At least 12 characters.')).toBeInTheDocument();
	expect(fetchFn).not.toHaveBeenCalled();
});

test('twelve plain lowercase characters are accepted', async () => {
	const fetchFn = stubFetch(201, {});
	const screen = render(RegisterForm, { onregistered: () => {} });

	await fillIn(screen, { Password: NO_COMPLEXITY });
	await screen.getByRole('button', create).click();

	await vi.waitFor(() => expect(fetchFn).toHaveBeenCalledOnce());
});

test('submit focuses the first field that failed', async () => {
	const screen = render(RegisterForm, { onregistered: () => {} });

	await screen.getByRole('button', create).click();

	await vi.waitFor(() =>
		expect(document.activeElement).toBe(screen.getByLabelText('Username').element())
	);
});

test('a server field code lands under its own field', async () => {
	stubFetch(400, { error: { code: 'INVALID_FULL_NAME', message: 'Name is too long' } });
	const screen = render(RegisterForm, { onregistered: () => {} });

	await fillIn(screen);
	await screen.getByRole('button', create).click();

	await expect.element(screen.getByText('Name is too long')).toBeInTheDocument();
	await expect.element(screen.getByLabelText('Full name')).toHaveAttribute('aria-invalid', 'true');
});

// ── in flight ──────────────────────────────────────────────────────────────

test('a second press cannot create two accounts', async () => {
	let release: (value: Response) => void = () => {};
	vi.stubGlobal(
		'fetch',
		vi.fn<typeof fetch>(() => new Promise<Response>((resolve) => (release = resolve)))
	);
	const screen = render(RegisterForm, { onregistered: () => {} });

	await fillIn(screen);
	const button = screen.getByRole('button', create);
	await button.click();

	await expect.element(button).toBeDisabled();

	release(new Response('{}', { status: 201 }));
});

// ── accessibility ──────────────────────────────────────────────────────────

test('has no accessibility violations', async () => {
	render(RegisterForm, { onregistered: () => {} });

	await expectNoA11yViolations(document.body, UNSTYLED_GEOMETRY);
});

test('has none in the collision state either', async () => {
	stubFetch(409, { error: { code: 'USER_ALREADY_EXISTS' } });
	const screen = render(RegisterForm, { onregistered: () => {} });

	await fillIn(screen);
	await screen.getByRole('button', create).click();
	await expect
		.element(screen.getByText("There's already an account for that address."))
		.toBeInTheDocument();

	await expectNoA11yViolations(document.body, UNSTYLED_GEOMETRY);
});
