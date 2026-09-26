import { expect, test, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { expectNoA11yViolations } from '$lib/shared/test/a11y';
import MediaPage from './media-page.svelte';

/**
 * `/studio/media` — Screen / Media library 69:242.
 *
 * Listing is by target, so the screen is a set of per-target views rather than
 * one library. §01: "scope it as a per-target picker instead."
 */

/** These specs render without the stylesheet, so hit areas are not real. */
const UNSTYLED_GEOMETRY = { rules: { 'target-size': { enabled: false } } };

const ITEMS = [
	{
		media_id: 'm-1',
		original_filename: 'hexagonal-layout.png',
		role: 'cover' as const,
		status: 'ready' as const,
		alt_text: 'A hexagonal diagram',
		src: 'https://signed.example.test/m-1'
	},
	{
		media_id: 'm-2',
		original_filename: 'tokopedia-arch.png',
		role: 'inline' as const,
		status: 'ready' as const,
		alt_text: '',
		src: 'https://signed.example.test/m-2'
	},
	{
		media_id: 'm-3',
		original_filename: 'screenshot-4.png',
		role: 'inline' as const,
		status: 'failed' as const,
		alt_text: '',
		src: null
	}
];

const json = (body: unknown, status = 200) => Response.json(body, { status });

const props = (over: Record<string, unknown> = {}) => ({
	items: ITEMS,
	scope: 'blog_post',
	onchanged: () => {},
	onquery: () => {},
	fetchFn: vi.fn(async () => json({ data: null })) as unknown as typeof fetch,
	...over
});

const sent = (mock: { mock: { calls: unknown[] } }) =>
	mock.mock.calls as unknown as [string, RequestInit | undefined][];

test('names each image by the file it came from', async () => {
	const screen = render(MediaPage, props());

	await expect.element(screen.getByText('hexagonal-layout.png')).toBeInTheDocument();
});

test('an image with no description asks for one', async () => {
	// The whole reason this screen earns its place: it is where missing
	// descriptions can be seen as a set rather than one at a time.
	const screen = render(MediaPage, props());

	await expect.element(screen.getByText('No alt text — add one')).toBeInTheDocument();
});

test('a described image says what it is for instead', async () => {
	const screen = render(MediaPage, props());

	await expect.element(screen.getByText('Alt text set · cover')).toBeInTheDocument();
});

test('scoping is by target, because the listing is', async () => {
	const onquery = vi.fn();
	const screen = render(MediaPage, props({ onquery }));

	await screen.getByRole('button', { name: 'Projects' }).click();

	// The archive setting rides along, so switching target does not silently
	// drop it.
	expect(onquery).toHaveBeenCalledWith('project', false);
});

test('the target being shown is marked, and is not a button to itself', async () => {
	const screen = render(MediaPage, props());

	await expect
		.element(screen.getByRole('button', { name: 'Posts' }))
		.toHaveAttribute('aria-current', 'true');
});

test('editing a description opens with what is there', async () => {
	const screen = render(MediaPage, props());

	await screen.getByRole('button', { name: 'Edit alt text for hexagonal-layout.png' }).click();

	await expect
		.element(screen.getByRole('textbox', { name: 'Alt text' }))
		.toHaveValue('A hexagonal diagram');
});

test('a corrected description is sent as a patch, and only that field', async () => {
	const fetchFn = vi.fn(async () => json({ data: null }));
	const onchanged = vi.fn();
	const screen = render(
		MediaPage,
		props({ fetchFn: fetchFn as unknown as typeof fetch, onchanged })
	);

	await screen.getByRole('button', { name: 'Edit alt text for tokopedia-arch.png' }).click();
	await screen.getByRole('textbox', { name: 'Alt text' }).fill('The architecture diagram');
	await screen.getByRole('button', { name: 'Save' }).click();

	await vi.waitFor(() => expect(onchanged).toHaveBeenCalled());
	const [url, init] = sent(fetchFn)[0];
	expect(url).toBe('/api/media/m-2');
	expect(JSON.parse(String(init?.body))).toEqual({ alt_text: 'The architecture diagram' });
});

test('a description cannot be emptied from here either', async () => {
	const screen = render(MediaPage, props());

	await screen.getByRole('button', { name: 'Edit alt text for hexagonal-layout.png' }).click();
	await screen.getByRole('textbox', { name: 'Alt text' }).fill('  ');

	await expect.element(screen.getByRole('button', { name: 'Save' })).toBeDisabled();
});

test('archiving asks first, because an image may be on a live page', async () => {
	const fetchFn = vi.fn(async () => json({ data: null }));
	const screen = render(MediaPage, props({ fetchFn: fetchFn as unknown as typeof fetch }));

	await screen.getByRole('button', { name: 'Archive hexagonal-layout.png' }).click();

	expect(sent(fetchFn).filter(([, init]) => init?.method === 'DELETE')).toHaveLength(0);
	await expect.element(screen.getByRole('button', { name: 'Archive image' })).toBeInTheDocument();
});

test('confirming archives it, and it is the reversible rung', async () => {
	const fetchFn = vi.fn(async () => json({ data: null }));
	const onchanged = vi.fn();
	const screen = render(
		MediaPage,
		props({ fetchFn: fetchFn as unknown as typeof fetch, onchanged })
	);

	await screen.getByRole('button', { name: 'Archive hexagonal-layout.png' }).click();
	await screen.getByRole('button', { name: 'Archive image' }).click();

	await vi.waitFor(() => expect(onchanged).toHaveBeenCalled());
	const [url, init] = sent(fetchFn)[0];
	expect(url).toBe('/api/media/m-1');
	expect(init?.method).toBe('DELETE');
});

test('a target with nothing in it says so', async () => {
	const screen = render(MediaPage, props({ items: [] }));

	await expect.element(screen.getByText('No images on your posts yet.')).toBeInTheDocument();
});

test('has no accessibility violations', async () => {
	render(MediaPage, props());

	await expectNoA11yViolations(document.body, UNSTYLED_GEOMETRY);
});

/**
 * Archived tiles — unblocked by `?include_deleted=true` and `deleted_at`.
 *
 * "Archived media could already be restored and purged, but not *found*: the
 * listing excluded it and the row carried no sign it existed, so a Restore
 * control had nothing to act on."
 *
 * One grid, as 69:242 draws it: archived tiles sit among the live ones and are
 * told apart by `deleted_at`.
 */

const ARCHIVED = {
	media_id: 'm-9',
	original_filename: 'old-diagram.png',
	role: 'inline' as const,
	status: 'ready' as const,
	alt_text: 'An older diagram',
	src: null,
	deleted_at: '2026-09-01T09:00:00Z'
};

test('showing archived is offered, and asks the listing for them', async () => {
	const onquery = vi.fn();
	const screen = render(MediaPage, props({ onquery }));

	await screen.getByRole('checkbox', { name: 'Showing archived' }).click();

	expect(onquery).toHaveBeenCalledWith('blog_post', true);
});

test('an archived tile is marked, and offers the two rungs it has left', async () => {
	const screen = render(MediaPage, props({ items: [...ITEMS, ARCHIVED], archived: true }));

	// Scoped: "Showing archived" is on the same screen and contains the word.
	await expect.element(screen.getByText('Archived', { exact: true })).toBeInTheDocument();
	await expect
		.element(screen.getByRole('button', { name: 'Restore old-diagram.png' }))
		.toBeInTheDocument();
	await expect
		.element(screen.getByRole('button', { name: 'Purge old-diagram.png' }))
		.toBeInTheDocument();
});

test('an archived tile is not asked to fix its alt text', async () => {
	// It is not on anything. Describing it better is not the job in front of
	// whoever is looking at it.
	const screen = render(MediaPage, props({ items: [ARCHIVED], archived: true }));

	expect(
		screen.getByRole('button', { name: 'Edit alt text for old-diagram.png' }).elements()
	).toHaveLength(0);
});

test('restoring puts it back, and tells the page', async () => {
	const fetchFn = vi.fn(async () => json({ data: null }));
	const onchanged = vi.fn();
	const screen = render(
		MediaPage,
		props({
			items: [ARCHIVED],
			archived: true,
			onchanged,
			fetchFn: fetchFn as unknown as typeof fetch
		})
	);

	await screen.getByRole('button', { name: 'Restore old-diagram.png' }).click();

	await vi.waitFor(() => expect(onchanged).toHaveBeenCalled());
	const [url, init] = sent(fetchFn)[0];
	expect(url).toBe('/api/media/m-9/restore');
	expect(init?.method).toBe('POST');
});

test('purging asks harder than archiving did, because it is the rung that stays', async () => {
	const fetchFn = vi.fn(async () => json({ data: null }));
	const screen = render(
		MediaPage,
		props({ items: [ARCHIVED], archived: true, fetchFn: fetchFn as unknown as typeof fetch })
	);

	await screen.getByRole('button', { name: 'Purge old-diagram.png' }).click();

	expect(sent(fetchFn)).toHaveLength(0);
	await expect
		.element(screen.getByText('Purge this image? This one does not come back.'))
		.toBeInTheDocument();
});

test('confirming a purge is the hard delete', async () => {
	const fetchFn = vi.fn(async () => json({ data: null }));
	const onchanged = vi.fn();
	const screen = render(
		MediaPage,
		props({
			items: [ARCHIVED],
			archived: true,
			onchanged,
			fetchFn: fetchFn as unknown as typeof fetch
		})
	);

	await screen.getByRole('button', { name: 'Purge old-diagram.png' }).click();
	await screen.getByRole('button', { name: 'Purge for good' }).click();

	await vi.waitFor(() => expect(onchanged).toHaveBeenCalled());
	const [url, init] = sent(fetchFn)[0];
	expect(url).toBe('/api/media/m-9/hard');
	expect(init?.method).toBe('DELETE');
});
