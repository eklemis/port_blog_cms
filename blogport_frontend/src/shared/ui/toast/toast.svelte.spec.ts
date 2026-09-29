import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { createRawSnippet, flushSync, tick } from 'svelte';
import { render } from 'vitest-browser-svelte';
import { expectNoA11yViolations } from '$lib/shared/test/a11y';
import Toast from './toast.svelte';

/**
 * Confirmations for things a person cannot see happen.
 *
 * §06 is explicit about the boundary: an autosaving screen reports itself in
 * one fixed place near the title, and a toast is not that. This is for the
 * consequence that happened somewhere else — a post going live at an address.
 */

const UNSTYLED_GEOMETRY = { rules: { 'target-size': { enabled: false } } };

const action = createRawSnippet(() => ({ render: () => '<a href="/jane/blog/x">View</a>' }));

beforeEach(() => vi.useFakeTimers({ shouldAdvanceTime: true }));
afterEach(() => vi.useRealTimers());

test('says what happened', async () => {
	const screen = render(Toast, { message: 'Published.', onclose: () => {} });

	await expect.element(screen.getByText('Published.')).toBeInTheDocument();
});

test('carries what to do next, when there is something', async () => {
	const screen = render(Toast, { message: 'Published.', action, onclose: () => {} });

	await expect.element(screen.getByRole('link', { name: 'View' })).toBeInTheDocument();
});

test('it goes away on its own after eight seconds', async () => {
	// §08's number. Long enough to read a link and follow it, short enough not
	// to sit over the screen.
	//
	// Three things have to be true before the clock is touched, or this measures
	// the wrong interval.
	//
	// `shouldAdvanceTime` is off so real elapsed time — awaiting, rendering, a
	// slow CI machine — cannot move the fake clock underneath the assertions.
	//
	// The component arms its timer in an `$effect`, which runs after mount. In
	// browser mode `render` can return *before* mount, and `flushSync` only
	// flushes effects that are already scheduled — so it can flush nothing at
	// all. `await tick()` is what gets us past mount; it is a microtask, so a
	// frozen clock cannot stall it the way a polling wait would.
	//
	// And then the precondition is asserted rather than hoped for: with no
	// timer armed, advancing the clock would prove nothing and this test would
	// fail for a reason that has nothing to do with eight seconds.
	vi.useFakeTimers({ shouldAdvanceTime: false });

	const closed: boolean[] = [];
	render(Toast, { message: 'Published.', onclose: () => closed.push(true) });
	await tick();
	flushSync();

	expect(vi.getTimerCount(), 'the toast armed its timer').toBe(1);

	await vi.advanceTimersByTimeAsync(7999);
	expect(closed).toHaveLength(0);

	await vi.advanceTimersByTimeAsync(1);
	expect(closed).toHaveLength(1);
});

test('it can be dismissed before then', async () => {
	const closed: boolean[] = [];
	const screen = render(Toast, { message: 'Published.', onclose: () => closed.push(true) });

	await screen.getByRole('button', { name: 'Dismiss' }).click();

	expect(closed).toHaveLength(1);
});

test('the clock does not run out while someone is reading it', async () => {
	// A toast that vanishes under the pointer on its way to the link is the
	// reason people distrust them.
	vi.useFakeTimers({ shouldAdvanceTime: false });

	const closed: boolean[] = [];
	const screen = render(Toast, {
		message: 'Published.',
		action,
		onclose: () => closed.push(true)
	});
	await tick();
	flushSync();

	// This test cannot tell "hovering stopped the clock" from "the clock never
	// started" by looking at `closed` — an empty array satisfies both, which is
	// how the race next door went unnoticed here. So the two states are
	// asserted directly, either side of the pointer.
	expect(vi.getTimerCount(), 'the toast was counting before the pointer arrived').toBe(1);

	await screen.getByText('Published.').hover();
	flushSync();

	expect(vi.getTimerCount(), 'hovering stopped the count').toBe(0);

	await vi.advanceTimersByTimeAsync(10_000);

	expect(closed).toHaveLength(0);
});

test('it is announced politely, because nothing was lost', async () => {
	render(Toast, { message: 'Published.', onclose: () => {} });

	expect(document.querySelector('[role="status"]')).not.toBeNull();
	expect(document.querySelector('[role="alert"]')).toBeNull();
});

test('has no accessibility violations', async () => {
	render(Toast, { message: 'Published.', action, onclose: () => {} });

	await expectNoA11yViolations(document.body, UNSTYLED_GEOMETRY);
});
