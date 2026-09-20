import { expect, test, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { expectNoA11yViolations } from '$lib/shared/test/a11y';
import CreateProjectForm from './create-project-form.svelte';

/**
 * Adding a project. There is no frame for this screen; §02 and §03 specify it.
 *
 * The fact that shapes it: a project has no draft state, so pressing the button
 * publishes. §02 asks for that to be said on the button rather than discovered.
 */

/** These specs render without the stylesheet, so hit areas are not real. */
const UNSTYLED_GEOMETRY = { rules: { 'target-size': { enabled: false } } };

const created = () => Response.json({ data: { id: 'p-9' } }, { status: 201 });
const free = () => Response.json({ available: true, suggestion: null });

const props = (over: Record<string, unknown> = {}) => ({
	oncreated: () => {},
	fetchFn: vi.fn(async (url: string) =>
		String(url).includes('slug-available') ? free() : created()
	) as unknown as typeof fetch,
	...over
});

const sent = (mock: { mock: { calls: unknown[] } }) =>
	mock.mock.calls as unknown as [string, RequestInit][];

test('says that adding one publishes it, before the button is pressed', async () => {
	// §02: "Say so on the create button — 'Add project' with the helper
	// 'Projects are visible on your public page as soon as they're added.'"
	const screen = render(CreateProjectForm, props());

	await expect
		.element(screen.getByText('Projects are visible on your public page as soon as they’re added.'))
		.toBeInTheDocument();
});

test('suggests an address from the title, without taking the field over', async () => {
	// The slug becomes the public URL. §01: normalise as they type rather than
	// silently rewriting the value after save.
	const screen = render(CreateProjectForm, props());

	await screen.getByRole('textbox', { name: 'Title' }).fill('Blogport CMS');

	await expect
		.element(screen.getByRole('textbox', { name: 'Address' }))
		.toHaveValue('blogport-cms');
});

test('an address typed by hand is not overwritten by the title', async () => {
	const screen = render(CreateProjectForm, props());

	await screen.getByRole('textbox', { name: 'Address' }).fill('my-cms');
	await screen.getByRole('textbox', { name: 'Title' }).fill('Blogport CMS');

	await expect.element(screen.getByRole('textbox', { name: 'Address' })).toHaveValue('my-cms');
});

test('creates the project and hands its id back', async () => {
	const oncreated = vi.fn();
	const fetchFn = vi.fn(async (url: string) =>
		String(url).includes('slug-available') ? free() : created()
	);
	const screen = render(
		CreateProjectForm,
		props({ oncreated, fetchFn: fetchFn as unknown as typeof fetch })
	);

	await screen.getByRole('textbox', { name: 'Title' }).fill('Blogport CMS');
	await screen.getByRole('textbox', { name: 'Description' }).fill('A portfolio CMS.');
	await screen.getByRole('button', { name: 'Add project' }).click();

	await vi.waitFor(() => expect(oncreated).toHaveBeenCalledWith('p-9'));

	const post = sent(fetchFn).find(([url]) => url === '/api/projects');
	expect(JSON.parse(String(post?.[1].body))).toEqual({
		title: 'Blogport CMS',
		slug: 'blogport-cms',
		description: 'A portfolio CMS.'
	});
});

test('a taken address is said under the address, with the free one beside it', async () => {
	// §02: "SLUG_ALREADY_EXISTS · 409. Same handling as posts: suggest, don't
	// discard." The typed value stays; the suggestion is offered.
	const fetchFn = vi.fn(async (url: string) =>
		String(url).includes('slug-available')
			? Response.json({ available: false, suggestion: 'blogport-cms-2' })
			: created()
	);
	const screen = render(CreateProjectForm, props({ fetchFn: fetchFn as unknown as typeof fetch }));

	await screen.getByRole('textbox', { name: 'Title' }).fill('Blogport CMS');

	await expect.element(screen.getByText('That address is already in use.')).toBeInTheDocument();
	await expect
		.element(screen.getByRole('button', { name: 'Use blogport-cms-2' }))
		.toBeInTheDocument();
});

test('taking the suggestion puts it in the field', async () => {
	const fetchFn = vi.fn(async (url: string) =>
		String(url).includes('slug-available')
			? Response.json({ available: false, suggestion: 'blogport-cms-2' })
			: created()
	);
	const screen = render(CreateProjectForm, props({ fetchFn: fetchFn as unknown as typeof fetch }));

	await screen.getByRole('textbox', { name: 'Title' }).fill('Blogport CMS');
	await screen.getByRole('button', { name: 'Use blogport-cms-2' }).click();

	await expect
		.element(screen.getByRole('textbox', { name: 'Address' }))
		.toHaveValue('blogport-cms-2');
});

test('an empty form is not submitted', async () => {
	// §05: an untouched required field is not an error until submit — and submit
	// validates everything rather than asking the server to.
	const fetchFn = vi.fn(async () => created());
	const screen = render(CreateProjectForm, props({ fetchFn: fetchFn as unknown as typeof fetch }));

	await screen.getByRole('button', { name: 'Add project' }).click();

	expect(sent(fetchFn).filter(([url]) => url === '/api/projects')).toHaveLength(0);
	await expect.element(screen.getByText('A title is required.')).toBeInTheDocument();
});

test('a refusal keeps every field, because retyping it is the punishment', async () => {
	const fetchFn = vi.fn(async (url: string) =>
		String(url).includes('slug-available')
			? free()
			: Response.json({ error: { code: 'INTERNAL_ERROR' } }, { status: 500 })
	);
	const screen = render(CreateProjectForm, props({ fetchFn: fetchFn as unknown as typeof fetch }));

	await screen.getByRole('textbox', { name: 'Title' }).fill('Blogport CMS');
	await screen.getByRole('textbox', { name: 'Description' }).fill('A portfolio CMS.');
	await screen.getByRole('button', { name: 'Add project' }).click();

	await expect.element(screen.getByRole('status')).toBeInTheDocument();
	await expect.element(screen.getByRole('textbox', { name: 'Title' })).toHaveValue('Blogport CMS');
});

test('has no accessibility violations', async () => {
	render(CreateProjectForm, props());

	await expectNoA11yViolations(document.body, UNSTYLED_GEOMETRY);
});
