import { expect, test, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { expectNoA11yViolations } from '$lib/shared/test/a11y';
import { TITLE_MAX } from '$lib/entities/post';
import CreatePostForm from './create-post-form.svelte';

/**
 * "New post" — J4 step one, the smallest thing that can exist.
 *
 * The post must exist before it can have a cover image, so this screen asks for
 * the three fields the API requires and then gets out of the way.
 */

const UNSTYLED_GEOMETRY = { rules: { 'target-size': { enabled: false } } };

/** Answers the slug check as free unless a test says otherwise. */
function backend(answers: Record<string, unknown> = {}) {
	return vi.fn<typeof fetch>(async (input, init) => {
		const url = String(input);
		const key = Object.keys(answers).find((candidate) => url.startsWith(candidate));
		const creating = init?.method === 'POST';
		const fallback = creating ? { id: 'post-1' } : { available: true, slug: 'x', suggestion: null };
		const body = key ? answers[key] : fallback;
		const status = creating && !key ? 201 : 200;
		return new Response(JSON.stringify(body), {
			status,
			headers: { 'content-type': 'application/json' }
		});
	});
}

const props = (over: Record<string, unknown> = {}) => ({
	oncreated: () => {},
	fetchFn: backend(),
	...over
});

test('the address is derived from the title, so nobody has to invent one', async () => {
	const screen = render(CreatePostForm, props());

	await screen.getByRole('textbox', { name: 'Title' }).fill('Building a CMS in Rust');

	await expect
		.element(screen.getByRole('textbox', { name: 'Web address' }))
		.toHaveValue('building-a-cms-in-rust');
});

test('once someone writes their own address, the title stops overwriting it', async () => {
	// Retyping a slug that keeps being replaced is the worst kind of fight to
	// have with a form.
	const screen = render(CreatePostForm, props());

	await screen.getByRole('textbox', { name: 'Title' }).fill('First title');
	await screen.getByRole('textbox', { name: 'Web address' }).fill('my-own-address');
	await screen.getByRole('textbox', { name: 'Title' }).fill('A completely new title');

	await expect
		.element(screen.getByRole('textbox', { name: 'Web address' }))
		.toHaveValue('my-own-address');
});

test('the counter appears before the cap, not at it', async () => {
	// J4: a live counter from 160 characters onward rather than a rejection at
	// submit.
	const screen = render(CreatePostForm, props());

	await screen.getByRole('textbox', { name: 'Title' }).fill('a'.repeat(159));
	expect(screen.getByText(`159 / ${TITLE_MAX}`).elements()).toHaveLength(0);

	await screen.getByRole('textbox', { name: 'Title' }).fill('a'.repeat(161));
	await expect.element(screen.getByText(`161 / ${TITLE_MAX}`)).toBeInTheDocument();
});

test('a taken address is reported before submitting, with the free one offered', async () => {
	const fetchFn = backend({
		'/api/blog/slug-available': { available: false, slug: 'taken', suggestion: 'taken-2' }
	});
	const screen = render(CreatePostForm, props({ fetchFn }));

	await screen.getByRole('textbox', { name: 'Title' }).fill('Taken');

	await expect.element(screen.getByRole('button', { name: 'Use taken-2' })).toBeInTheDocument();
});

test('taking the suggestion puts it in the field', async () => {
	const fetchFn = backend({
		'/api/blog/slug-available': { available: false, slug: 'taken', suggestion: 'taken-2' }
	});
	const screen = render(CreatePostForm, props({ fetchFn }));

	await screen.getByRole('textbox', { name: 'Title' }).fill('Taken');
	await screen.getByRole('button', { name: 'Use taken-2' }).click();

	await expect.element(screen.getByRole('textbox', { name: 'Web address' })).toHaveValue('taken-2');
});

test('an empty form is not sent, and says what is missing', async () => {
	const fetchFn = backend();
	const screen = render(CreatePostForm, props({ fetchFn }));

	await screen.getByRole('button', { name: 'Create draft' }).click();

	await expect.element(screen.getByText('A title is required.')).toBeInTheDocument();
	expect(fetchFn.mock.calls.filter((call) => call[1]?.method === 'POST')).toHaveLength(0);
});

test('the body is required, because the API refuses a post without one', async () => {
	// `validate_content` rejects an empty body outright — so a genuinely blank
	// draft is not a thing that can exist. See the PR.
	const screen = render(CreatePostForm, props());

	await screen.getByRole('textbox', { name: 'Title' }).fill('A post');
	await screen.getByRole('button', { name: 'Create draft' }).click();

	await expect.element(screen.getByText('The post needs something in it.')).toBeInTheDocument();
});

test('creating hands the id on, because the editor is the next screen', async () => {
	const created: string[] = [];
	const screen = render(CreatePostForm, props({ oncreated: (id: string) => created.push(id) }));

	await screen.getByRole('textbox', { name: 'Title' }).fill('A post');
	await screen.getByRole('textbox', { name: 'Post' }).fill('The first line.');
	await screen.getByRole('button', { name: 'Create draft' }).click();

	await vi.waitFor(() => expect(created).toEqual(['post-1']));
});

test('a collision on save keeps everything that was typed', async () => {
	// J4: never lose the draft body to a slug collision.
	const fetchFn = vi.fn<typeof fetch>(async (input, init) => {
		if (init?.method === 'POST') {
			return new Response(
				JSON.stringify({ error: { code: 'SLUG_ALREADY_EXISTS', message: 'taken' } }),
				{ status: 409, headers: { 'content-type': 'application/json' } }
			);
		}
		return new Response(JSON.stringify({ available: true, slug: 'x', suggestion: null }), {
			status: 200,
			headers: { 'content-type': 'application/json' }
		});
	});
	const screen = render(CreatePostForm, props({ fetchFn }));

	await screen.getByRole('textbox', { name: 'Title' }).fill('A post');
	await screen.getByRole('textbox', { name: 'Post' }).fill('The first line.');
	await screen.getByRole('button', { name: 'Create draft' }).click();

	await expect.element(screen.getByText('That address is already in use.')).toBeInTheDocument();
	await expect
		.element(screen.getByRole('textbox', { name: 'Post' }))
		.toHaveValue('The first line.');
});

test('has no accessibility violations', async () => {
	render(CreatePostForm, props());

	await expectNoA11yViolations(document.body, UNSTYLED_GEOMETRY);
});
