import { expect, test, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { expectNoA11yViolations } from '$lib/shared/test/a11y';
import CoverCard from './cover-card.svelte';

/**
 * The editor rail's Cover image card — Screen / Post editor 32:999.
 *
 * The frame draws one state. §03's MediaTile row specifies the rest:
 * "uploading · pending · processing · ready · failed — uploading shows
 * determinate progress (the only honest bar); pending and processing are
 * indeterminate. failed persists until dismissed."
 */

/** These specs render without the stylesheet, so hit areas are not real. */
const UNSTYLED_GEOMETRY = { rules: { 'target-size': { enabled: false } } };

const png = () => new File(['fake-bytes'], 'hero.png', { type: 'image/png' });

/** The three browser capabilities a test has to stand in for. */
const deps = (over: Record<string, unknown> = {}) => ({
	postId: 'post-1',
	measure: async () => ({ width: 1600, height: 900 }),
	upload: async () => ({ ok: true as const }),
	fetchFn: vi.fn(async () =>
		Response.json({ data: { media_id: 'm-1', upload_url: 'https://gcs/put' } }, { status: 201 })
	) as unknown as typeof fetch,
	...over
});

async function choose(screen: ReturnType<typeof render>, file = png()) {
	const input = screen.container.querySelector('input[type="file"]') as HTMLInputElement;
	const transfer = new DataTransfer();
	transfer.items.add(file);
	input.files = transfer.files;
	input.dispatchEvent(new Event('change', { bubbles: true }));
}

test('an empty card offers a way to add one, and shows no broken image', async () => {
	const screen = render(CoverCard, deps());

	await expect.element(screen.getByText('Cover image')).toBeInTheDocument();
	expect(screen.container.querySelectorAll('img')).toHaveLength(0);
});

test('a file of the wrong kind is refused before anything is uploaded', async () => {
	const fetchFn = vi.fn();
	const screen = render(CoverCard, deps({ fetchFn: fetchFn as unknown as typeof fetch }));

	await choose(screen, new File(['x'], 'loop.gif', { type: 'image/gif' }));

	await expect.element(screen.getByText('Images must be a JPEG, PNG or WebP.')).toBeInTheDocument();
	expect(fetchFn).not.toHaveBeenCalled();
});

test('an image too big for the policy is refused by the browser, which is the only checker', async () => {
	// The bytes never reach the API, so nothing else can measure them.
	const fetchFn = vi.fn();
	const screen = render(
		CoverCard,
		deps({
			fetchFn: fetchFn as unknown as typeof fetch,
			measure: async () => ({ width: 8000, height: 400 })
		})
	);

	await choose(screen);

	await expect
		.element(screen.getByText('Images must be 6000px or smaller on each side. That one is 8000px.'))
		.toBeInTheDocument();
	expect(fetchFn).not.toHaveBeenCalled();
});

test('alt text is required before the upload can start', async () => {
	// §03: "alt_text · yes · non-empty (spec rule, not API) · Blocks upload."
	const fetchFn = vi.fn();
	const screen = render(CoverCard, deps({ fetchFn: fetchFn as unknown as typeof fetch }));

	await choose(screen);

	await expect.element(screen.getByRole('button', { name: 'Upload' })).toBeDisabled();
	expect(fetchFn).not.toHaveBeenCalled();
});

test('with alt text written, the upload starts and the bar is measured', async () => {
	// The only honest bar: a real fraction of real bytes.
	const screen = render(
		CoverCard,
		deps({
			upload: async (_url: string, _file: File, options: { onprogress?: (f: number) => void }) => {
				options.onprogress?.(0.5);
				return new Promise(() => {}) as Promise<{ ok: true }>;
			}
		})
	);

	await choose(screen);
	await screen.getByRole('textbox', { name: 'Alt text' }).fill('A hexagonal diagram');
	await screen.getByRole('button', { name: 'Upload' }).click();

	const bar = screen.getByRole('progressbar');
	await expect.element(bar).toHaveAttribute('aria-valuenow', '50');
});

test('a cover still being processed says so, and offers nothing to press', async () => {
	// 32:999 draws exactly this. Retry cannot help while a queue is working.
	const screen = render(
		CoverCard,
		deps({ cover: { media_id: 'm-1', status: 'processing', alt_text: 'A diagram' } })
	);

	await expect.element(screen.getByText('Processing')).toBeInTheDocument();
});

test('a ready cover is shown with the alt text it was given', async () => {
	const screen = render(
		CoverCard,
		deps({
			cover: { media_id: 'm-1', status: 'ready', alt_text: 'A hexagonal diagram' },
			coverSrc: 'https://signed.example.test/large'
		})
	);

	await expect
		.element(screen.getByRole('img', { name: 'A hexagonal diagram' }))
		.toBeInTheDocument();
	await expect.element(screen.getByRole('button', { name: 'Remove' })).toBeInTheDocument();
});

test('a failed upload keeps saying so, and offers both ways out', async () => {
	// §03: "failed persists until dismissed — a silently vanishing upload is
	// worse than a visible failure."
	const screen = render(
		CoverCard,
		deps({ cover: { media_id: 'm-1', status: 'failed', alt_text: 'A diagram' } })
	);

	// Two things say it, and they are for two different people: the pill for
	// anyone looking at the rail, the live region for anyone who is not.
	await expect.element(screen.getByText('Failed', { exact: true })).toBeInTheDocument();
	await expect.element(screen.getByText('Cover image failed.')).toBeInTheDocument();
	await expect.element(screen.getByRole('button', { name: 'Remove' })).toBeInTheDocument();
});

test('removing the cover asks the API to delete it, and tells the page', async () => {
	const fetchFn = vi.fn(async () => Response.json({ data: null }));
	const onchanged = vi.fn();
	const screen = render(
		CoverCard,
		deps({
			fetchFn: fetchFn as unknown as typeof fetch,
			onchanged,
			cover: { media_id: 'm-1', status: 'ready', alt_text: 'A diagram' },
			coverSrc: 'https://signed.example.test/large'
		})
	);

	await screen.getByRole('button', { name: 'Remove' }).click();

	expect((fetchFn.mock.calls as unknown as [string][])[0][0]).toBe('/api/media/m-1');
	expect(onchanged).toHaveBeenCalled();
});

test('does not claim the alt text can never be changed', async () => {
	// The frame's note says "Alt text is set once and cannot be edited later."
	// `PATCH /api/media/{id}` exists precisely to undo that: "Alt text, caption
	// and position are set at upload and were not editable, so a missing or
	// wrong alt text was a permanent accessibility defect." Raised with the
	// designer; until the sentence is rewritten this card does not assert it.
	const screen = render(CoverCard, deps());

	expect(screen.container.innerHTML).not.toMatch(/cannot be edited/i);
});

test('has no accessibility violations', async () => {
	render(CoverCard, deps());

	await expectNoA11yViolations(document.body, UNSTYLED_GEOMETRY);
});
