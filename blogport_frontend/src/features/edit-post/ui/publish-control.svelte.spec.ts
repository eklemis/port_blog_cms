import { expect, test } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { expectNoA11yViolations } from '$lib/shared/test/a11y';
import PublishControl from './publish-control.svelte';

/**
 * Publish, or schedule — J4 step five.
 *
 * "One control with two outcomes." The thing it must say plainly is that a
 * future date goes live with no further call: nothing will happen on screen
 * when it does, so the screen has to say so before the fact.
 */

const UNSTYLED_GEOMETRY = { rules: { 'target-size': { enabled: false } } };

const now = new Date('2026-09-08T12:00:00Z');

const props = (over: Record<string, unknown> = {}) => ({
	publishedAt: null,
	busy: false,
	onpublish: () => {},
	onunpublish: () => {},
	now,
	...over
});

test('a draft offers to publish', async () => {
	const screen = render(PublishControl, props());

	await expect.element(screen.getByRole('button', { name: 'Publish' })).toBeInTheDocument();
});

test('publishing now sends no date, and the caller reads that as now', async () => {
	const asked: (string | null)[] = [];
	const screen = render(
		PublishControl,
		props({ onpublish: (at: string | null) => asked.push(at) })
	);

	await screen.getByRole('button', { name: 'Publish' }).click();

	expect(asked).toEqual([null]);
});

test('scheduling says plainly that nothing else will happen', async () => {
	// The whole risk of this control: a future date goes live with no further
	// call, and nothing happens on screen when it does.
	const screen = render(PublishControl, props());

	await screen.getByRole('button', { name: 'Schedule instead' }).click();

	await expect.element(screen.getByText(/goes live on its own/)).toBeInTheDocument();
});

test('a scheduled time is sent as an instant, not as what the clock said', async () => {
	const asked: (string | null)[] = [];
	const screen = render(
		PublishControl,
		props({ onpublish: (at: string | null) => asked.push(at) })
	);

	await screen.getByRole('button', { name: 'Schedule instead' }).click();
	await screen.getByLabelText('Goes live').fill('2026-09-09T09:00');
	await screen.getByRole('button', { name: 'Schedule' }).click();

	expect(asked).toHaveLength(1);
	expect(new Date(asked[0] as string).getTime()).toBe(new Date('2026-09-09T09:00').getTime());
});

test('a time in the past is refused rather than silently publishing', async () => {
	// "Schedule" and "publish now" are two outcomes of one control, and the
	// person picked the one that is not now.
	const asked: (string | null)[] = [];
	const screen = render(
		PublishControl,
		props({ onpublish: (at: string | null) => asked.push(at) })
	);

	await screen.getByRole('button', { name: 'Schedule instead' }).click();
	await screen.getByLabelText('Goes live').fill('2026-09-07T09:00');
	await screen.getByRole('button', { name: 'Schedule' }).click();

	expect(asked).toHaveLength(0);
	await expect.element(screen.getByText('Pick a time in the future.')).toBeInTheDocument();
});

// ── once it is live ────────────────────────────────────────────────────────

test('a live post says so, and offers the way back', async () => {
	const screen = render(PublishControl, props({ publishedAt: '2026-09-01T09:00:00Z' }));

	await expect.element(screen.getByText('Live')).toBeInTheDocument();
	await expect.element(screen.getByRole('button', { name: 'Unpublish' })).toBeInTheDocument();
	// Exact: "Unpublish" contains "Publish", and a substring match would find
	// the very button whose absence is the point.
	expect(screen.getByRole('button', { name: 'Publish', exact: true }).elements()).toHaveLength(0);
});

test('a scheduled post is not called live, because it is not', async () => {
	const screen = render(PublishControl, props({ publishedAt: '2026-09-09T09:00:00Z' }));

	await expect.element(screen.getByText('Scheduled')).toBeInTheDocument();
});

test('unpublishing says what it costs before it is pressed', async () => {
	const screen = render(PublishControl, props({ publishedAt: '2026-09-01T09:00:00Z' }));

	await expect.element(screen.getByText(/public address stops working/)).toBeInTheDocument();
});

test('nothing can be pressed twice while a call is in the air', async () => {
	const screen = render(PublishControl, props({ busy: true }));

	await expect.element(screen.getByRole('button', { name: 'Publish' })).toBeDisabled();
});

test('has no accessibility violations', async () => {
	render(PublishControl, props());

	await expectNoA11yViolations(document.body, UNSTYLED_GEOMETRY);
});

test('nor does the live state', async () => {
	render(PublishControl, props({ publishedAt: '2026-09-01T09:00:00Z' }));

	await expectNoA11yViolations(document.body, UNSTYLED_GEOMETRY);
});
