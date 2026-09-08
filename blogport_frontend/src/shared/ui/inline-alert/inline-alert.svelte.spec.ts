import { expect, test } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { expectNoA11yViolations } from '$lib/shared/test/a11y';
import InlineAlert from './inline-alert.svelte';

/**
 * The form-level message, and the six handling classes colour-mapped.
 *
 * The reason this is one component rather than a `<p>` per form: every form
 * had rolled its own, and four of the five hid the live region while it was
 * empty — which is the one thing the page that reasoned about it out loud says
 * not to do.
 */

const UNSTYLED_GEOMETRY = { rules: { 'target-size': { enabled: false } } };

test('says what it was given', async () => {
	const screen = render(InlineAlert, { message: "That email and password don't match." });

	await expect
		.element(screen.getByText("That email and password don't match."))
		.toBeInTheDocument();
});

test('the live region exists before it has anything to say', async () => {
	// A region inserted at the moment of failure is often missed, and one that
	// is display:none while empty has left the accessibility tree — a change
	// that both fills and reveals it is not reliably announced.
	render(InlineAlert, { message: undefined });

	const region = document.querySelector('[role="status"]');
	expect(region, 'nothing to announce into').not.toBeNull();
	expect(region?.textContent?.trim()).toBe('');
});

test('the region takes no colour and no space while it is empty', async () => {
	// It stays in the tree, but an empty element must not leave a coloured gap
	// in the middle of a form.
	render(InlineAlert, { message: undefined });

	const region = document.querySelector('[role="status"]');
	expect(region?.className).not.toMatch(/text-st-/);
});

test('polite, because a refusal has lost nothing', async () => {
	// Assertive is reserved for loss (Accessibility Spec §09). A sign-in that
	// did not go through, a name already taken, a rate limit — none of them
	// has destroyed anything, and interrupting says otherwise. `role="alert"`
	// is assertive by definition, so it is the wrong role here however the
	// message reads.
	render(InlineAlert, { message: 'Too many attempts. Try again in 42 minutes.' });

	expect(document.querySelector('[role="status"]')).not.toBeNull();
	expect(document.querySelector('[role="alert"]')).toBeNull();
});

// ── the six classes ────────────────────────────────────────────────────────

test('a rejected value is red', async () => {
	const screen = render(InlineAlert, { message: 'That is not an email address.', kind: 'field' });

	await expect
		.element(screen.getByText('That is not an email address.'))
		.toHaveClass(/text-st-danger/);
});

test('a fork, a gate and a wait are amber, not red', async () => {
	// None of them failed. Red on a rate limit tells someone their work broke.
	for (const kind of ['collision', 'gate', 'wait'] as const) {
		const screen = render(InlineAlert, { message: `A ${kind} message`, kind });

		await expect.element(screen.getByText(`A ${kind} message`)).toHaveClass(/text-st-inflight/);
	}
});

test('an unclassified message is treated as their side breaking', async () => {
	// Which is the default the mapping itself takes, for the same reason.
	const screen = render(InlineAlert, { message: 'Something went wrong on our side.' });

	await expect
		.element(screen.getByText('Something went wrong on our side.'))
		.toHaveClass(/text-st-danger/);
});

test('has no accessibility violations', async () => {
	render(InlineAlert, { message: 'Verify your email to start publishing.', kind: 'gate' });

	await expectNoA11yViolations(document.body, UNSTYLED_GEOMETRY);
});
