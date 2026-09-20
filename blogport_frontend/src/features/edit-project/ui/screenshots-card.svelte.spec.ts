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

/**
 * Adding one — 70:258 draws "+ Upload" beside the heading.
 *
 * The same flow as the post's cover, through the same entity call with three
 * values changed: target `project`, role `screenshot`, and a position at the
 * end of the gallery.
 */

const png = () => new File(['bytes'], 'settings.png', { type: 'image/png' });

const uploadable = (over: Record<string, unknown> = {}) =>
	props({
		projectId: 'p-1',
		measure: async () => ({ width: 1600, height: 900 }),
		upload: async () => ({ ok: true as const }),
		...over
	});

async function choose(screen: ReturnType<typeof render>, file = png()) {
	const input = screen.container.querySelector('input[type="file"]') as HTMLInputElement;
	const transfer = new DataTransfer();
	transfer.items.add(file);
	input.files = transfer.files;
	input.dispatchEvent(new Event('change', { bubbles: true }));
}

test('offers a way to add one', async () => {
	const screen = render(ScreenshotsCard, uploadable());

	await expect.element(screen.getByText('+ Upload')).toBeInTheDocument();
});

test('a file the policy refuses never reaches the network', async () => {
	// The bytes go straight to storage, so the browser is the only checker.
	const fetchFn = vi.fn();
	const screen = render(
		ScreenshotsCard,
		uploadable({ fetchFn: fetchFn as unknown as typeof fetch })
	);

	await choose(screen, new File(['x'], 'loop.gif', { type: 'image/gif' }));

	await expect.element(screen.getByText('Images must be a JPEG, PNG or WebP.')).toBeInTheDocument();
	expect(fetchFn).not.toHaveBeenCalled();
});

test('alt text blocks the upload until it is written', async () => {
	// §03: "alt_text · yes · non-empty (spec rule, not API) · Blocks upload."
	const screen = render(ScreenshotsCard, uploadable());

	await choose(screen);

	await expect.element(screen.getByRole('button', { name: 'Add' })).toBeDisabled();
});

test('the upload declares the project, the role and a place at the end', async () => {
	// ADR 0008: lowercase `screenshot`, snake_case `project`. "Any client
	// sending the capitalized forms breaks."
	const fetchFn = vi.fn(async () =>
		Response.json({ data: { media_id: 'new-1', upload_url: 'https://gcs' } }, { status: 201 })
	);
	const screen = render(
		ScreenshotsCard,
		uploadable({ fetchFn: fetchFn as unknown as typeof fetch })
	);

	await choose(screen);
	await screen.getByRole('textbox', { name: 'Alt text' }).fill('The settings screen');
	await screen.getByRole('button', { name: 'Add' }).click();

	await vi.waitFor(() => expect(fetchFn).toHaveBeenCalled());

	const [url, init] = (fetchFn.mock.calls as unknown as [string, RequestInit][])[0];
	expect(url).toBe('/api/media/upload-url');
	expect(JSON.parse(String(init.body))).toMatchObject({
		attachment_target: 'project',
		attachment_target_id: 'p-1',
		role: 'screenshot',
		alt_text: 'The settings screen',
		position: 3
	});
});

test('a finished upload hands back to the page rather than guessing the row', async () => {
	// The new row's filename and processing state are the server's to report.
	const onchanged = vi.fn();
	const fetchFn = vi.fn(async () =>
		Response.json({ data: { media_id: 'new-1', upload_url: 'https://gcs' } }, { status: 201 })
	);
	const screen = render(
		ScreenshotsCard,
		uploadable({ onchanged, fetchFn: fetchFn as unknown as typeof fetch })
	);

	await choose(screen);
	await screen.getByRole('textbox', { name: 'Alt text' }).fill('The settings screen');
	await screen.getByRole('button', { name: 'Add' }).click();

	await vi.waitFor(() => expect(onchanged).toHaveBeenCalled());
});

test('a card with no project cannot upload, and does not pretend to', async () => {
	// The screenshots card renders inside the editor, which always has one —
	// but a missing id would otherwise POST an attachment to nothing.
	const screen = render(ScreenshotsCard, props({ projectId: undefined }));

	expect(screen.getByText('+ Upload').elements()).toHaveLength(0);
});

test('an upload the server refuses does not tell the page anything changed', async () => {
	// Nothing was attached, so re-reading the gallery would show the same rows
	// and the refusal would vanish with the reload.
	const onchanged = vi.fn();
	const fetchFn = vi.fn(async () =>
		Response.json({ error: { code: 'FILE_TOO_LARGE' } }, { status: 400 })
	);
	const screen = render(
		ScreenshotsCard,
		uploadable({ onchanged, fetchFn: fetchFn as unknown as typeof fetch })
	);

	await choose(screen);
	await screen.getByRole('textbox', { name: 'Alt text' }).fill('The settings screen');
	await screen.getByRole('button', { name: 'Add' }).click();

	await vi.waitFor(() => expect(fetchFn).toHaveBeenCalled());
	expect(onchanged).not.toHaveBeenCalled();
});
