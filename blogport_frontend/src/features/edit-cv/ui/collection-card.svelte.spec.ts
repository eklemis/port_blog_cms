import { createRawSnippet } from 'svelte';
import { expect, test, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { expectNoA11yViolations } from '$lib/shared/test/a11y';
import CollectionCard from './collection-card.svelte';

/**
 * The shape shared by the résumé's four short collections.
 *
 * No frame shows one open — 69:2 draws them as a name, a count and "+ Add" —
 * so the behaviour is borrowed from the experience list on the same screen
 * rather than invented: rows collapse to a summary, one opens at a time.
 */

/** These specs render without the stylesheet, so hit areas are not real. */
const UNSTYLED_GEOMETRY = { rules: { 'target-size': { enabled: false } } };

const SKILLS = [
	{ title: 'Backend', description: 'Rust, Postgres' },
	{ title: 'Infra', description: 'GCP, Terraform' }
];

type Skill = { title: string; description: string };

/**
 * The row's fields, supplied the way a real caller supplies them.
 *
 * A `.svelte` fixture would be a component with no spec of its own, which the
 * house guard rightly refuses — so the snippet is built here instead, and the
 * card is tested directly rather than through whichever collection happened to
 * use it first.
 */
type Update = (patch: Partial<Skill>) => void;

/**
 * The card is generic over the row, and a render call erases that to `unknown`
 * — so the test supplies one concrete type and says where it narrows, rather
 * than the whole spec being written against `unknown`.
 */
const fields = createRawSnippet<[unknown, unknown]>((item, update) => ({
	render: () => `<label>Title <input type="text" /></label>`,
	setup: (element: Element) => {
		const input = element.querySelector('input') as HTMLInputElement;
		input.value = (item() as Skill).title;
		input.addEventListener('input', () => (update() as Update)({ title: input.value }));
	}
}));

const props = (over: Record<string, unknown> = {}) => ({
	label: 'Core skills',
	items: SKILLS,
	blank: () => ({ title: '', description: '' }),
	summary: (item: unknown) => (item as Skill).title || 'New entry',
	fields,
	onchange: () => {},
	...over
});

test('each row collapses to its own summary', async () => {
	const screen = render(CollectionCard, props());

	await expect.element(screen.getByText('Backend')).toBeInTheDocument();
	await expect.element(screen.getByText('Infra')).toBeInTheDocument();
});

test('nothing is open until something is opened', async () => {
	const screen = render(CollectionCard, props());

	expect(screen.getByRole('textbox', { name: 'Title' }).elements()).toHaveLength(0);
});

test('one opens at a time, as on the experience list', async () => {
	const screen = render(CollectionCard, props());

	await screen.getByRole('button', { name: 'Edit Backend' }).click();
	await screen.getByRole('button', { name: 'Edit Infra' }).click();

	expect(screen.getByRole('textbox', { name: 'Title' }).elements()).toHaveLength(1);
	await expect.element(screen.getByRole('textbox', { name: 'Title' })).toHaveValue('Infra');
});

test('editing reports the whole list, because the list is replaced wholesale', async () => {
	const onchange = vi.fn();
	const screen = render(CollectionCard, props({ onchange }));

	await screen.getByRole('button', { name: 'Edit Backend' }).click();
	await screen.getByRole('textbox', { name: 'Title' }).fill('Backend systems');

	await vi.waitFor(() => expect(onchange).toHaveBeenCalled());
	const next = onchange.mock.calls.at(-1)?.[0];
	expect(next).toHaveLength(2);
	expect(next[0].title).toBe('Backend systems');
	expect(next[1].title).toBe('Infra');
});

test('adding opens the new row, because nobody adds one to leave it shut', async () => {
	const onchange = vi.fn();
	const screen = render(CollectionCard, props({ onchange }));

	await screen.getByRole('button', { name: 'Add to core skills' }).click();

	await expect.element(screen.getByRole('textbox', { name: 'Title' })).toHaveValue('');
	expect(onchange.mock.calls.at(-1)?.[0]).toHaveLength(3);
});

test('a row can be removed while it is open', async () => {
	const onchange = vi.fn();
	const screen = render(CollectionCard, props({ onchange }));

	await screen.getByRole('button', { name: 'Edit Backend' }).click();
	await screen.getByRole('button', { name: 'Remove Backend' }).click();

	await vi.waitFor(() => expect(onchange).toHaveBeenCalled());
	expect(onchange.mock.calls.at(-1)?.[0]).toHaveLength(1);
});

test('an empty collection says so rather than showing an empty box', async () => {
	const screen = render(CollectionCard, props({ items: [] }));

	await expect.element(screen.getByText('0 entries')).toBeInTheDocument();
});

test('has no accessibility violations', async () => {
	render(CollectionCard, props());

	await expectNoA11yViolations(document.body, UNSTYLED_GEOMETRY);
});
