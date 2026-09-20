import { expect, test, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { expectNoA11yViolations } from '$lib/shared/test/a11y';
import TopicsPage from './topics-page.svelte';

/**
 * `/studio/topics` — the shared vocabulary.
 *
 * Screen / Topics 70:382 exists; Figma was unavailable this session, so this is
 * built from §02's journey and §03's field rules rather than from the frame.
 */

/** These specs render without the stylesheet, so hit areas are not real. */
const UNSTYLED_GEOMETRY = { rules: { 'target-size': { enabled: false } } };

const TOPICS = [
	{ id: 't-1', title: 'Rust', description: 'Systems work, mostly backend.' },
	{ id: 't-2', title: 'SvelteKit', description: '' }
];

const json = (body: unknown, status = 200) => Response.json(body, { status });

const props = (over: Record<string, unknown> = {}) => ({
	topics: TOPICS,
	onchanged: () => {},
	fetchFn: vi.fn(async () => json({ data: { posts: 6, projects: 2 } })) as unknown as typeof fetch,
	...over
});

const sent = (mock: { mock: { calls: unknown[] } }) =>
	mock.mock.calls as unknown as [string, RequestInit | undefined][];

test('lists the vocabulary with what each word means', async () => {
	// §02: "title and description in the same popover — a taxonomy of bare
	// words stops being useful at about fifteen entries."
	const screen = render(TopicsPage, props());

	await expect.element(screen.getByText('Rust')).toBeInTheDocument();
	await expect.element(screen.getByText('Systems work, mostly backend.')).toBeInTheDocument();
});

test('a topic can be renamed, and the screen never suggests retagging', async () => {
	// §02's journey says "Retire, don't rename... A typo is unfixable" and
	// prescribes create-retag-retire. PATCH /api/topics/{id} has since shipped:
	// "The topic keeps its id, so everything tagged with it follows the new name
	// automatically — nothing needs retagging."
	const screen = render(TopicsPage, props());

	await expect.element(screen.getByRole('button', { name: 'Rename Rust' })).toBeInTheDocument();
	expect(screen.container.innerHTML).not.toMatch(/retag/i);
});

test('renaming opens with the name that is there', async () => {
	const screen = render(TopicsPage, props());

	await screen.getByRole('button', { name: 'Rename Rust' }).click();

	await expect.element(screen.getByRole('textbox', { name: 'Title' })).toHaveValue('Rust');
});

test('a rename sends the new title and tells the page', async () => {
	const onchanged = vi.fn();
	const fetchFn = vi.fn(async () => json({ data: { id: 't-1' } }));
	const screen = render(
		TopicsPage,
		props({ onchanged, fetchFn: fetchFn as unknown as typeof fetch })
	);

	await screen.getByRole('button', { name: 'Rename Rust' }).click();
	await screen.getByRole('textbox', { name: 'Title' }).fill('Rust and systems');
	await screen.getByRole('button', { name: 'Save' }).click();

	await vi.waitFor(() => expect(onchanged).toHaveBeenCalled());
	expect(JSON.parse(String(sent(fetchFn)[0][1]?.body))).toEqual({ title: 'Rust and systems' });
});

test('retiring counts what it is on before it asks', async () => {
	// §02: "Count it first. GET /api/topics/{id}/usage returns the real numbers,
	// so the dialog can say 'Retire «Rust»? It's on 6 posts and 2 projects.'
	// Never drop a topic off eight pages silently."
	const screen = render(TopicsPage, props());

	await screen.getByRole('button', { name: 'Retire Rust' }).click();

	await expect
		.element(screen.getByText('Retire «Rust»? It’s on 6 posts and 2 projects.'))
		.toBeInTheDocument();
});

test('asking is not doing — the topic stays until it is confirmed', async () => {
	const fetchFn = vi.fn(async () => json({ data: { posts: 6, projects: 2 } }));
	const screen = render(TopicsPage, props({ fetchFn: fetchFn as unknown as typeof fetch }));

	await screen.getByRole('button', { name: 'Retire Rust' }).click();

	expect(sent(fetchFn).filter(([, init]) => init?.method === 'DELETE')).toHaveLength(0);
});

test('an unused topic is retired without a warning it does not deserve', async () => {
	const fetchFn = vi.fn(async () => json({ data: { posts: 0, projects: 0 } }));
	const screen = render(TopicsPage, props({ fetchFn: fetchFn as unknown as typeof fetch }));

	await screen.getByRole('button', { name: 'Retire SvelteKit' }).click();

	await expect
		.element(screen.getByText('Retire «SvelteKit»? Nothing is using it.'))
		.toBeInTheDocument();
});

test('confirming retires it, and it is a soft delete', async () => {
	const onchanged = vi.fn();
	const fetchFn = vi.fn(async (url: string, init?: RequestInit) =>
		init?.method === 'DELETE' ? json({ data: null }) : json({ data: { posts: 0, projects: 0 } })
	);
	const screen = render(
		TopicsPage,
		props({ onchanged, fetchFn: fetchFn as unknown as typeof fetch })
	);

	await screen.getByRole('button', { name: 'Retire Rust' }).click();
	await screen.getByRole('button', { name: 'Retire topic' }).click();

	await vi.waitFor(() => expect(onchanged).toHaveBeenCalled());
	expect(
		sent(fetchFn).some(([url, init]) => url === '/api/topics/t-1' && init?.method === 'DELETE')
	).toBe(true);
});

test('an author with no vocabulary yet is told what one is for', async () => {
	const screen = render(TopicsPage, props({ topics: [] }));

	await expect.element(screen.getByText('No topics yet.')).toBeInTheDocument();
});

test('has no accessibility violations', async () => {
	render(TopicsPage, props());

	await expectNoA11yViolations(document.body, UNSTYLED_GEOMETRY);
});
