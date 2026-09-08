import { expect, test } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { expectNoA11yViolations } from '$lib/shared/test/a11y';
import Button from './button.svelte';

/**
 * The reference spec. Note what it asserts: things a person can observe —
 * rendered text, roles, what a click does, what a screen reader is told.
 * It never reaches for internal state. Copy this shape.
 *
 * These run in a real browser (vite.config.ts routes *.svelte.spec.ts to the
 * `client` project); plain *.spec.ts runs in node instead.
 */

test('renders its label as an accessible name', async () => {
	const screen = render(Button, { label: 'Publish' });
	await expect.element(screen.getByRole('button', { name: 'Publish' })).toBeInTheDocument();
});

test('calls onclick when pressed', async () => {
	let clicked = 0;
	const screen = render(Button, { label: 'Publish', onclick: () => clicked++ });

	await screen.getByRole('button', { name: 'Publish' }).click();

	expect(clicked).toBe(1);
});

test('does not fire while disabled', async () => {
	let clicked = 0;
	const screen = render(Button, {
		label: 'Publish',
		disabled: true,
		onclick: () => clicked++
	});

	await expect.element(screen.getByRole('button')).toBeDisabled();
	expect(clicked).toBe(0);
});

test('a disabled button explains itself', async () => {
	const screen = render(Button, {
		label: 'Publish',
		disabled: true,
		disabledReason: 'Add a cover image before publishing.'
	});

	// The reason has to reach assistive tech, not just the eye — a disabled
	// control whose reason is invisible is a dead end with extra steps.
	await expect
		.element(screen.getByRole('button'))
		.toHaveAccessibleDescription('Add a cover image before publishing.');
});

test('keeps its accessible name while loading', async () => {
	const screen = render(Button, { label: 'Publish', loading: true });

	// Still "Publish", never "Loading…" — a name that changes mid-action reads
	// as a different control.
	const button = screen.getByRole('button', { name: 'Publish' });
	await expect.element(button).toHaveAttribute('aria-busy', 'true');
	await expect.element(button).toBeDisabled();
});

test.each(['primary', 'secondary', 'ghost', 'danger'] as const)(
	'%s has no accessibility violations',
	async (kind) => {
		// One per test on purpose. Rendering all four into one body stacks them
		// flush together and axe reports `target-size` for adjacent targets — an
		// artefact of the harness, not of the component, which real layouts space out.
		render(Button, { label: 'Publish', kind });

		await expectNoA11yViolations();
	}
);

test('a disabled button is still accessible', async () => {
	render(Button, {
		label: 'Publish',
		disabled: true,
		disabledReason: 'Add a cover image before publishing.'
	});

	await expectNoA11yViolations();
});

test('is not a submit control unless asked', async () => {
	const screen = render(Button, { label: 'Publish' });

	// The default matters: a bare <button> inside a form submits it, which is
	// how an icon toggle becomes an accidental save.
	await expect.element(screen.getByRole('button')).toHaveAttribute('type', 'button');
});

test('can be the submit control of a form', async () => {
	// A form with two inputs has no implicit submission without one, so Enter
	// from a single-line input would stop working. Forms Spec §06.
	const screen = render(Button, { label: 'Sign in', type: 'submit' });

	await expect.element(screen.getByRole('button')).toHaveAttribute('type', 'submit');
});

test('renders a link when it is given somewhere to go', async () => {
	// Some of these controls navigate. A button that navigates is not a button:
	// it cannot be opened in a new tab, and it is announced as the wrong thing.
	const screen = render(Button, { label: 'Sign in instead', href: '/auth/login' });

	await expect
		.element(screen.getByRole('link', { name: 'Sign in instead' }))
		.toHaveAttribute('href', '/auth/login');
	expect(screen.getByRole('button').elements()).toHaveLength(0);
});

test('a link still looks like the kind it was asked for', async () => {
	const screen = render(Button, {
		label: 'Sign in instead',
		href: '/auth/login',
		kind: 'secondary'
	});

	await expect
		.element(screen.getByRole('link', { name: 'Sign in instead' }))
		.toHaveClass(/border-arch-line-control/);
});

test('a link has no accessibility violations', async () => {
	render(Button, { label: 'Sign in instead', href: '/auth/login', kind: 'secondary' });

	await expectNoA11yViolations();
});
