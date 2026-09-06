import { expect, test } from 'vitest';
import { createRawSnippet } from 'svelte';
import { render } from 'vitest-browser-svelte';
import { expectNoA11yViolations } from '$lib/shared/test/a11y';
import Field from './field.svelte';

/**
 * Field — the five states from Forms & Interaction Spec §04, and the Field row
 * of Accessibility Spec §08.
 *
 * Everything asserted here is observable: what the label announces, what the
 * screen reader is told about help and error, what a blur does — and, just as
 * load-bearing, what a keystroke does not do.
 */

// ── default ────────────────────────────────────────────────────────────────

test('the label names the input through a real for/id pair', async () => {
	const screen = render(Field, { label: 'Email' });

	// getByLabelText resolves through label association, so this passes only if
	// the pair is real. Placeholder-as-label would not be found.
	await expect.element(screen.getByLabelText('Email')).toBeInTheDocument();
});

test('the placeholder is an example, and never the label', async () => {
	const screen = render(Field, { label: 'Email', placeholder: 'jane@example.com' });

	const input = screen.getByLabelText('Email');
	await expect.element(input).toHaveAttribute('placeholder', 'jane@example.com');
	await expect.element(input).toHaveValue('');
});

// ── help ───────────────────────────────────────────────────────────────────

test('help text is announced with the input, not merely drawn near it', async () => {
	const screen = render(Field, {
		label: 'Password',
		type: 'password',
		help: 'At least 12 characters.'
	});

	await expect
		.element(screen.getByLabelText('Password'))
		.toHaveAccessibleDescription('At least 12 characters.');
});

// ── filled ─────────────────────────────────────────────────────────────────

test('a value passed in renders as the field contents', async () => {
	const screen = render(Field, { label: 'Email', value: 'jane@example.com' });

	await expect.element(screen.getByLabelText('Email')).toHaveValue('jane@example.com');
});

test('typing fills the field', async () => {
	const screen = render(Field, { label: 'Email' });

	const input = screen.getByLabelText('Email');
	await input.fill('jane@example.com');

	// That the typed value reaches the *parent* through `bind:value` is not
	// observable from a bare mount — the round trip is asserted end-to-end in
	// features/sign-in, where a real form reads it and posts it.
	await expect.element(input).toHaveValue('jane@example.com');
});

// ── error ──────────────────────────────────────────────────────────────────

test('an error marks the input invalid and is announced', async () => {
	const screen = render(Field, {
		label: 'Email',
		error: 'That doesn’t look like an email address.'
	});

	const input = screen.getByLabelText('Email');
	await expect.element(input).toHaveAttribute('aria-invalid', 'true');
	await expect
		.element(input)
		.toHaveAccessibleDescription('That doesn’t look like an email address.');
});

test('an error replaces help text and never stacks with it', async () => {
	const screen = render(Field, {
		label: 'Password',
		type: 'password',
		help: 'At least 12 characters.',
		error: 'At least 12 characters.'
	});

	// The same string on purpose: if both slots rendered, the accessible
	// description would carry it twice. Forms Spec §04 — "Error replaces help
	// text, never stacks with it."
	await expect
		.element(screen.getByLabelText('Password'))
		.toHaveAccessibleDescription('At least 12 characters.');
});

test('without an error the input is not marked invalid', async () => {
	const screen = render(Field, { label: 'Email' });

	await expect.element(screen.getByLabelText('Email')).not.toHaveAttribute('aria-invalid');
});

// ── disabled ───────────────────────────────────────────────────────────────

test('a disabled field cannot be typed into', async () => {
	const screen = render(Field, { label: 'Username', value: 'janedoe', disabled: true });

	await expect.element(screen.getByLabelText('Username')).toBeDisabled();
});

// ── required ───────────────────────────────────────────────────────────────

test('required uses the attribute, not an asterisk alone', async () => {
	const screen = render(Field, { label: 'Email', required: true });

	await expect.element(screen.getByLabelText('Email')).toBeRequired();
});

// ── validation timing ──────────────────────────────────────────────────────

test('blur reports once the field loses focus, and a keystroke never does', async () => {
	let blurs = 0;
	const screen = render(Field, { label: 'Email', onblur: () => blurs++ });

	const input = screen.getByLabelText('Email');
	await input.fill('jane@example.com');
	expect(blurs, 'a keystroke is not a validation trigger').toBe(0);

	input.element().dispatchEvent(new FocusEvent('blur', { bubbles: false }));
	expect(blurs).toBe(1);
});

test('forwards each keystroke, so a caller can tell a touched field from an untouched one', async () => {
	let keystrokes = 0;
	const screen = render(Field, { label: 'Email', oninput: () => keystrokes++ });

	await screen.getByLabelText('Email').fill('jane');

	// Knowing the field was typed in is what lets the caller hold its error back
	// until blur — an untouched empty field is not an error yet.
	expect(keystrokes).toBeGreaterThan(0);
});

// ── password reveal ────────────────────────────────────────────────────────

test('a password field carries a reveal toggle, never a confirm twin', async () => {
	const screen = render(Field, { label: 'Password', type: 'password', value: 'a-real-password' });

	const input = screen.getByLabelText('Password');
	await expect.element(input).toHaveAttribute('type', 'password');

	await screen.getByRole('button', { name: 'Show' }).click();

	await expect.element(input).toHaveAttribute('type', 'text');
	await expect.element(screen.getByRole('button', { name: 'Hide' })).toBeInTheDocument();
});

test('the reveal toggle never submits the surrounding form', async () => {
	const screen = render(Field, { label: 'Password', type: 'password' });

	// A bare <button> inside a form defaults to type=submit, which would make
	// revealing a password sign you in.
	await expect
		.element(screen.getByRole('button', { name: 'Show' }))
		.toHaveAttribute('type', 'button');
});

test('a non-password field has no reveal toggle', async () => {
	const screen = render(Field, { label: 'Email' });

	expect(screen.getByRole('button').elements()).toHaveLength(0);
});

// ── label-row action ───────────────────────────────────────────────────────

test('renders an action beside the label when one is given', async () => {
	const screen = render(Field, {
		label: 'Password',
		type: 'password',
		labelAction: createRawSnippet(() => ({
			render: () => '<a href="/auth/forgot">Forgot password?</a>'
		}))
	});

	await expect.element(screen.getByRole('link', { name: 'Forgot password?' })).toBeInTheDocument();
});

// ── accessibility ──────────────────────────────────────────────────────────

test('the default state has no accessibility violations', async () => {
	render(Field, { label: 'Email', type: 'email', placeholder: 'jane@example.com' });

	await expectNoA11yViolations();
});

test('the help state has no accessibility violations', async () => {
	render(Field, { label: 'Password', type: 'password', help: 'At least 12 characters.' });

	await expectNoA11yViolations();
});

test('the error state has no accessibility violations', async () => {
	render(Field, {
		label: 'Email',
		type: 'email',
		value: 'not-an-address',
		error: 'That doesn’t look like an email address.'
	});

	await expectNoA11yViolations();
});

test('the disabled state has no accessibility violations', async () => {
	render(Field, { label: 'Username', value: 'janedoe', disabled: true });

	await expectNoA11yViolations();
});

test('takes a caller-supplied id, so a form can move focus to it', async () => {
	// Submit "focuses the first invalid one" — which the form can only do if it
	// knows what to focus. Forms Spec §05.
	const screen = render(Field, { label: 'Email', id: 'signin-email' });

	await expect.element(screen.getByLabelText('Email')).toHaveAttribute('id', 'signin-email');
});

test('a caller-supplied id still reaches the description', async () => {
	const screen = render(Field, { label: 'Email', id: 'signin-email', help: 'Work or personal.' });

	await expect
		.element(screen.getByLabelText('Email'))
		.toHaveAccessibleDescription('Work or personal.');
});

test('a maximum length caps the field instead of reporting an overrun', async () => {
	// The copy table has a sentence for the minimum and none for the maximum. A
	// field that cannot overrun needs no message.
	const screen = render(Field, { label: 'Password', type: 'password', maxlength: 128 });

	await expect.element(screen.getByLabelText('Password')).toHaveAttribute('maxlength', '128');
});
