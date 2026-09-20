import { expect, test, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { expectNoA11yViolations } from '$lib/shared/test/a11y';
import ScreenshotsCard from './screenshots-card.svelte';

/**
 * Screen / Project editor 70:257 — the screenshots card, and the reorder that
 * `PatchMediaRequest.position` unblocked.
 *
 * §04 settles the interaction: "No drag-only interaction — media reordering
 * offers move up / move down alongside the drag." A reorder that works only by
 * dragging is a reorder some people cannot do.
 */

/** These specs render without the stylesheet, so hit areas are not real. */
const UNSTYLED_GEOMETRY = { rules: { 'target-size': { enabled: false } } };

const SHOTS = [
	{ media_id: 'm-1', original_filename: 'console-posts.png', src: null },
	{ media_id: 'm-2', original_filename: 'tailor-cv.png', src: null },
	{ media_id: 'm-3', original_filename: 'dark-mode.png', src: null }
];

const ok = () => Response.json({ data: null });

const props = (over: Record<string, unknown> = {}) => ({
	screenshots: SHOTS,
	fetchFn: vi.fn(async () => ok()) as unknown as typeof fetch,
	...over
});

const names = (screen: ReturnType<typeof render>) =>
	[...screen.container.querySelectorAll('[data-filename]')].map((el) => el.textContent?.trim());

test('lists the screenshots in the order they are in', async () => {
	const screen = render(ScreenshotsCard, props());

	expect(names(screen)).toEqual(['console-posts.png', 'tailor-cv.png', 'dark-mode.png']);
});

test('every row can be moved without a mouse', async () => {
	// §04's rule. The drag handle is an affordance, not the only way in.
	const screen = render(ScreenshotsCard, props());

	await expect
		.element(screen.getByRole('button', { name: 'Move tailor-cv.png up' }))
		.toBeInTheDocument();
	await expect
		.element(screen.getByRole('button', { name: 'Move tailor-cv.png down' }))
		.toBeInTheDocument();
});

test('moving a row down reorders the list on screen at once', async () => {
	// Optimistic: the request follows, and the list is put back if it fails.
	const screen = render(ScreenshotsCard, props());

	await screen.getByRole('button', { name: 'Move console-posts.png down' }).click();

	expect(names(screen)).toEqual(['tailor-cv.png', 'console-posts.png', 'dark-mode.png']);
});

test('only the rows that actually moved are sent', async () => {
	// Each is its own PATCH, so sending all three when two changed is a request
	// that can fail for nothing.
	const fetchFn = vi.fn(async () => ok());
	const screen = render(ScreenshotsCard, props({ fetchFn: fetchFn as unknown as typeof fetch }));

	await screen.getByRole('button', { name: 'Move console-posts.png down' }).click();

	await vi.waitFor(() => expect(fetchFn).toHaveBeenCalledTimes(2));

	const calls = fetchFn.mock.calls as unknown as [string, RequestInit][];
	const sent = calls.map(([url, init]) => [url, JSON.parse(String(init.body))]);

	expect(sent).toEqual([
		['/api/media/m-2', { position: 0 }],
		['/api/media/m-1', { position: 1 }]
	]);
});

test('the first row cannot be moved up, and says why', async () => {
	// §08: "Disabled carries the reason in aria-describedby, never a bare
	// disabled attribute with no explanation."
	const screen = render(ScreenshotsCard, props());

	const up = screen.getByRole('button', { name: 'Move console-posts.png up' });

	await expect.element(up).toBeDisabled();
	await expect.element(up).toHaveAttribute('aria-describedby');
});

test('the last row cannot be moved down', async () => {
	const screen = render(ScreenshotsCard, props());

	await expect
		.element(screen.getByRole('button', { name: 'Move dark-mode.png down' }))
		.toBeDisabled();
});

test('a move that the server refuses puts the list back', async () => {
	// The rail must not keep showing an order the server does not have.
	const fetchFn = vi.fn(async () =>
		Response.json({ error: { code: 'MEDIA_NOT_FOUND' } }, { status: 404 })
	);
	const screen = render(ScreenshotsCard, props({ fetchFn: fetchFn as unknown as typeof fetch }));

	await screen.getByRole('button', { name: 'Move console-posts.png down' }).click();

	await vi.waitFor(() =>
		expect(names(screen)).toEqual(['console-posts.png', 'tailor-cv.png', 'dark-mode.png'])
	);
	await expect.element(screen.getByRole('status')).toBeInTheDocument();
});

test('a move is announced, because the row it moved is not where the eye was', async () => {
	const screen = render(ScreenshotsCard, props());

	await screen.getByRole('button', { name: 'Move console-posts.png down' }).click();

	await expect
		.element(screen.getByText('console-posts.png moved to position 2 of 3.'))
		.toBeInTheDocument();
});

test('a project with no screenshots says so rather than showing an empty box', async () => {
	const screen = render(ScreenshotsCard, props({ screenshots: [] }));

	await expect.element(screen.getByText('No screenshots yet.')).toBeInTheDocument();
});

test('one screenshot has nothing to reorder against', async () => {
	// Two buttons that can never be pressed are two dead controls.
	const screen = render(ScreenshotsCard, props({ screenshots: [SHOTS[0]] }));

	expect(screen.getByRole('button').elements()).toHaveLength(0);
});

test('has no accessibility violations', async () => {
	render(ScreenshotsCard, props());

	await expectNoA11yViolations(document.body, UNSTYLED_GEOMETRY);
});
