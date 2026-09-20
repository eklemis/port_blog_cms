import { expect, test, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { expectNoA11yViolations } from '$lib/shared/test/a11y';
import TopicPicker from './topic-picker.svelte';

/**
 * The editor rail's topic control — the "+ Add" the frames draw beside the
 * chips.
 *
 * §03: "topics · combobox · Search + multi-select + inline create. Offers
 * 'Create «Rust»' when nothing matches."
 */

/** These specs render without the stylesheet, so hit areas are not real. */
const UNSTYLED_GEOMETRY = { rules: { 'target-size': { enabled: false } } };

const props = (over: Record<string, unknown> = {}) => ({
	attached: [{ id: 't-rust', title: 'Rust' }],
	available: [
		{ id: 't-rust', title: 'Rust' },
		{ id: 't-systems', title: 'Systems' },
		{ id: 't-arch', title: 'Architecture' }
	],
	...over
});

test('shows the topics the post has, each with a way off', async () => {
	const dropped: string[] = [];
	const screen = render(
		TopicPicker,
		props({ ondetach: (t: { id: string }) => dropped.push(t.id) })
	);

	await expect.element(screen.getByText('Rust')).toBeInTheDocument();
	await screen.getByRole('button', { name: 'Remove Rust' }).click();

	expect(dropped).toEqual(['t-rust']);
});

test('opens on Add, and offers the topics not already on the post', async () => {
	// Multi-select over the author's own topics. One already attached is not
	// offered again — attaching it twice is a request that can only fail.
	const screen = render(TopicPicker, props());

	await screen.getByRole('button', { name: 'Add a topic' }).click();

	await expect.element(screen.getByRole('button', { name: 'Systems' })).toBeInTheDocument();
	await expect.element(screen.getByRole('button', { name: 'Architecture' })).toBeInTheDocument();
	expect(screen.getByRole('button', { name: 'Rust', exact: true }).elements()).toHaveLength(0);
});

test('searching narrows the list to what was typed', async () => {
	const screen = render(TopicPicker, props());

	await screen.getByRole('button', { name: 'Add a topic' }).click();
	await screen.getByRole('textbox', { name: 'Search topics' }).fill('arch');

	await expect.element(screen.getByRole('button', { name: 'Architecture' })).toBeInTheDocument();
	expect(screen.getByRole('button', { name: 'Systems' }).elements()).toHaveLength(0);
});

test('picking one attaches it and leaves the list open for the next', async () => {
	const picked: string[] = [];
	const screen = render(TopicPicker, props({ onattach: (t: { id: string }) => picked.push(t.id) }));

	await screen.getByRole('button', { name: 'Add a topic' }).click();
	await screen.getByRole('button', { name: 'Systems' }).click();

	expect(picked).toEqual(['t-systems']);
	// Multi-select: the point is adding several without reopening each time.
	await expect.element(screen.getByRole('textbox', { name: 'Search topics' })).toBeInTheDocument();
});

test('offers to create the typed topic when nothing matches', async () => {
	// §03: "Offers 'Create «Rust»' when nothing matches."
	const made: string[] = [];
	const screen = render(TopicPicker, props({ oncreate: (title: string) => made.push(title) }));

	await screen.getByRole('button', { name: 'Add a topic' }).click();
	await screen.getByRole('textbox', { name: 'Search topics' }).fill('Postgres');
	await screen.getByRole('button', { name: 'Create “Postgres”' }).click();

	expect(made).toEqual(['Postgres']);
});

test('creating closes the picker, because the chip is the confirmation', async () => {
	// §03: "Create & attach · Chip attaches, popover closes."
	const screen = render(TopicPicker, props({ oncreate: () => {} }));

	await screen.getByRole('button', { name: 'Add a topic' }).click();
	await screen.getByRole('textbox', { name: 'Search topics' }).fill('Postgres');
	await screen.getByRole('button', { name: 'Create “Postgres”' }).click();

	await vi.waitFor(() =>
		expect(screen.getByRole('textbox', { name: 'Search topics' }).elements()).toHaveLength(0)
	);
});

test('does not offer to create something that already exists', async () => {
	const screen = render(TopicPicker, props());

	await screen.getByRole('button', { name: 'Add a topic' }).click();
	await screen.getByRole('textbox', { name: 'Search topics' }).fill('Systems');

	expect(screen.getByRole('button', { name: /^Create/ }).elements()).toHaveLength(0);
});

test('Escape closes it and gives focus back to what opened it', async () => {
	const screen = render(TopicPicker, props());
	const add = screen.getByRole('button', { name: 'Add a topic' });

	await add.click();
	await screen.getByRole('textbox', { name: 'Search topics' }).fill('a');
	await userEscape();

	await vi.waitFor(() =>
		expect(screen.getByRole('textbox', { name: 'Search topics' }).elements()).toHaveLength(0)
	);
	await expect.element(add).toHaveFocus();
});

async function userEscape() {
	document.activeElement?.dispatchEvent(
		new KeyboardEvent('keydown', { key: 'Escape', bubbles: true })
	);
}

test('a post with no topics says so rather than showing an empty row', async () => {
	const screen = render(TopicPicker, props({ attached: [] }));

	await expect.element(screen.getByText('No topics yet.')).toBeInTheDocument();
});

test('has no accessibility violations, open or closed', async () => {
	const screen = render(TopicPicker, props());

	await expectNoA11yViolations(document.body, UNSTYLED_GEOMETRY);

	await screen.getByRole('button', { name: 'Add a topic' }).click();

	await expectNoA11yViolations(document.body, UNSTYLED_GEOMETRY);
});

test('a partial match is offered rather than a near-duplicate of it', async () => {
	// §03 offers to create "when nothing matches". Typing "arch" against an
	// existing "Architecture" should offer Architecture, not a second topic
	// one character away from it.
	const screen = render(TopicPicker, props());

	await screen.getByRole('button', { name: 'Add a topic' }).click();
	await screen.getByRole('textbox', { name: 'Search topics' }).fill('arch');

	await expect.element(screen.getByRole('button', { name: 'Architecture' })).toBeInTheDocument();
	expect(screen.getByRole('button', { name: /^Create/ }).elements()).toHaveLength(0);
});

test('a topic already on the post is not offered for creating either', async () => {
	// It is filtered out of the offers, so without the exact-title check the
	// picker would offer to make a second topic with the same name.
	const screen = render(TopicPicker, props());

	await screen.getByRole('button', { name: 'Add a topic' }).click();
	await screen.getByRole('textbox', { name: 'Search topics' }).fill('Rust');

	expect(screen.getByRole('button', { name: /^Create/ }).elements()).toHaveLength(0);
});
