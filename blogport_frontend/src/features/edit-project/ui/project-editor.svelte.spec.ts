import { expect, test, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { expectNoA11yViolations } from '$lib/shared/test/a11y';
import ProjectEditor from './project-editor.svelte';

/** Screen / Project editor 70:188. */

/** These specs render without the stylesheet, so hit areas are not real. */
const UNSTYLED_GEOMETRY = { rules: { 'target-size': { enabled: false } } };

const PROJECT = {
	id: 'p-1',
	title: 'Blogport CMS',
	slug: 'blogport-cms',
	description: 'A portfolio CMS split across four services.',
	tech_stack: ['Rust', 'SvelteKit'],
	topics: [{ id: 't-1', title: 'Rust' }],
	repo_url: 'https://github.com/eklemis/port_blog_cms',
	live_demo_url: null,
	updated_at: '2026-09-18T09:00:00Z'
};

const ok = () => Response.json({ data: null });

const props = (over: Record<string, unknown> = {}) => ({
	project: PROJECT,
	username: 'janedoe',
	screenshots: [],
	availableTopics: [{ id: 't-1', title: 'Rust' }],
	fetchFn: vi.fn(async () => ok()) as unknown as typeof fetch,
	...over
});

const sent = (mock: { mock: { calls: unknown[] } }) =>
	mock.mock.calls as unknown as [string, RequestInit][];

test('opens with the project that was loaded', async () => {
	const screen = render(ProjectEditor, props());

	await expect.element(screen.getByRole('textbox', { name: 'Title' })).toHaveValue('Blogport CMS');
	await expect
		.element(screen.getByRole('textbox', { name: 'Description' }))
		.toHaveValue('A portfolio CMS split across four services.');
});

test('the address is a fact, not a field', async () => {
	// `CreateProjectRequest` takes a slug; `PatchProjectRequest` does not. §04:
	// "Read-only facts render as text, not disabled inputs." A bordered box
	// someone can click into and not change is worse than a sentence.
	const screen = render(ProjectEditor, props());

	await expect.element(screen.getByText('blogport-cms')).toBeInTheDocument();
	expect(screen.getByRole('textbox', { name: 'Address' }).elements()).toHaveLength(0);
});

test('leads to the page a reader would see', async () => {
	const screen = render(ProjectEditor, props());

	await expect
		.element(screen.getByRole('link', { name: 'View public page' }))
		.toHaveAttribute('href', '/janedoe/projects/blogport-cms');
});

test('saving sends only what was edited', async () => {
	// §02: "there is no full-replace PUT, so the editor is field-level
	// throughout and never sends an object it did not load first."
	const fetchFn = vi.fn(async () => ok());
	const screen = render(ProjectEditor, props({ fetchFn: fetchFn as unknown as typeof fetch }));

	await screen.getByRole('textbox', { name: 'Title' }).fill('Blogport');
	await screen.getByRole('button', { name: 'Save' }).click();

	await vi.waitFor(() => expect(fetchFn).toHaveBeenCalled());
	expect(JSON.parse(String(sent(fetchFn)[0][1].body))).toEqual({ title: 'Blogport' });
});

test('an emptied URL is cleared rather than saved as an empty string', async () => {
	const fetchFn = vi.fn(async () => ok());
	const screen = render(ProjectEditor, props({ fetchFn: fetchFn as unknown as typeof fetch }));

	await screen.getByRole('textbox', { name: 'Repository' }).fill('');
	await screen.getByRole('button', { name: 'Save' }).click();

	await vi.waitFor(() => expect(fetchFn).toHaveBeenCalled());
	expect(JSON.parse(String(sent(fetchFn)[0][1].body))).toEqual({ repo_url: null });
});

test('saving nothing asks the server for nothing', async () => {
	const fetchFn = vi.fn(async () => ok());
	const screen = render(ProjectEditor, props({ fetchFn: fetchFn as unknown as typeof fetch }));

	await screen.getByRole('button', { name: 'Save' }).click();

	expect(fetchFn).not.toHaveBeenCalled();
});

test('tech stack is a chip input, never a comma-separated field', async () => {
	// §03's field table: "An array as removable tokens. Enter or , commits;
	// Backspace on an empty input removes the last."
	const fetchFn = vi.fn(async () => ok());
	const screen = render(ProjectEditor, props({ fetchFn: fetchFn as unknown as typeof fetch }));

	const field = screen.getByRole('textbox', { name: 'Add to tech stack' });
	await field.fill('SeaORM');
	(field.element() as HTMLInputElement).dispatchEvent(
		new KeyboardEvent('keydown', { key: 'Enter', bubbles: true })
	);
	await screen.getByRole('button', { name: 'Save' }).click();

	await vi.waitFor(() => expect(fetchFn).toHaveBeenCalled());
	expect(JSON.parse(String(sent(fetchFn)[0][1].body))).toEqual({
		tech_stack: ['Rust', 'SvelteKit', 'SeaORM']
	});
});

test('a comma commits a chip too, because people type lists that way', async () => {
	const screen = render(ProjectEditor, props());

	const field = screen.getByRole('textbox', { name: 'Add to tech stack' });
	await field.fill('SeaORM,');

	await expect.element(screen.getByRole('button', { name: 'Remove SeaORM' })).toBeInTheDocument();
});

test('a chip can be taken off again', async () => {
	// "Rust" is both a stack entry and a topic on this project, which is
	// realistic — so the query is scoped to the one being tested.
	const fetchFn = vi.fn(async () => ok());
	const screen = render(ProjectEditor, props({ fetchFn: fetchFn as unknown as typeof fetch }));

	await screen
		.getByRole('group', { name: 'Tech stack' })
		.getByRole('button', { name: 'Remove Rust' })
		.click();
	await screen.getByRole('button', { name: 'Save' }).click();

	await vi.waitFor(() => expect(fetchFn).toHaveBeenCalled());
	expect(JSON.parse(String(sent(fetchFn)[0][1].body))).toEqual({ tech_stack: ['SvelteKit'] });
});

test('a save that fails says so and keeps every edit', async () => {
	const fetchFn = vi.fn(async () =>
		Response.json({ error: { code: 'INTERNAL_ERROR' } }, { status: 500 })
	);
	const screen = render(ProjectEditor, props({ fetchFn: fetchFn as unknown as typeof fetch }));

	await screen.getByRole('textbox', { name: 'Title' }).fill('Blogport');
	await screen.getByRole('button', { name: 'Save' }).click();

	await expect.element(screen.getByRole('status')).toBeInTheDocument();
	await expect.element(screen.getByRole('textbox', { name: 'Title' })).toHaveValue('Blogport');
});

test('carries the screenshots card, which is where order is decided', async () => {
	const screen = render(ProjectEditor, props());

	await expect.element(screen.getByRole('region', { name: 'Screenshots' })).toBeInTheDocument();
});

test('has no accessibility violations', async () => {
	render(ProjectEditor, props());

	await expectNoA11yViolations(document.body, UNSTYLED_GEOMETRY);
});

test('a stack entry containing a space is not two entries', async () => {
	// The comparison that decides whether to save must not join on a character
	// somebody can type: ["Actix Web"] and ["Actix", "Web"] are different lists.
	const fetchFn = vi.fn(async () => ok());
	const screen = render(
		ProjectEditor,
		props({
			fetchFn: fetchFn as unknown as typeof fetch,
			project: { ...PROJECT, tech_stack: ['Actix Web'] }
		})
	);

	await screen
		.getByRole('group', { name: 'Tech stack' })
		.getByRole('button', { name: 'Remove Actix Web' })
		.click();

	const field = screen.getByRole('textbox', { name: 'Add to tech stack' });
	await field.fill('Actix');
	(field.element() as HTMLInputElement).dispatchEvent(
		new KeyboardEvent('keydown', { key: 'Enter', bubbles: true })
	);
	await field.fill('Web');
	(field.element() as HTMLInputElement).dispatchEvent(
		new KeyboardEvent('keydown', { key: 'Enter', bubbles: true })
	);

	await screen.getByRole('button', { name: 'Save' }).click();

	await vi.waitFor(() => expect(fetchFn).toHaveBeenCalled());
	expect(JSON.parse(String(sent(fetchFn)[0][1].body))).toEqual({ tech_stack: ['Actix', 'Web'] });
});
