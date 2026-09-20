import { expect, test, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { expectNoA11yViolations } from '$lib/shared/test/a11y';
import ChipInput from './chip-input.svelte';

/**
 * §03's `chips` field type: "An array as removable tokens. Enter or , commits;
 * Backspace on an empty input removes the last. Never a comma-separated text
 * field."
 *
 * In the kit rather than in a feature because two of them need it — a project's
 * tech stack on both the create form and the editor — and slices in the same
 * layer may not import each other.
 */

/** These specs render without the stylesheet, so hit areas are not real. */
const UNSTYLED_GEOMETRY = { rules: { 'target-size': { enabled: false } } };

const press = (screen: ReturnType<typeof render>, key: string) => {
	const input = screen.container.querySelector('input') as HTMLInputElement;
	input.dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true }));
};

const props = (over: Record<string, unknown> = {}) => ({
	label: 'Tech stack',
	values: ['Rust', 'SvelteKit'],
	...over
});

test('shows each value as its own token', async () => {
	const screen = render(ChipInput, props());

	await expect.element(screen.getByText('Rust')).toBeInTheDocument();
	await expect.element(screen.getByText('SvelteKit')).toBeInTheDocument();
});

test('every token can be taken off by name', async () => {
	// One "Remove" per chip would name every control the same thing.
	const onchange = vi.fn();
	const screen = render(ChipInput, props({ onchange }));

	await screen.getByRole('button', { name: 'Remove Rust' }).click();

	expect(onchange).toHaveBeenCalledWith(['SvelteKit']);
});

test('Enter commits what has been typed', async () => {
	const onchange = vi.fn();
	const screen = render(ChipInput, props({ onchange }));

	await screen.getByRole('textbox', { name: 'Add to tech stack' }).fill('SeaORM');
	press(screen, 'Enter');

	expect(onchange).toHaveBeenCalledWith(['Rust', 'SvelteKit', 'SeaORM']);
});

test('a comma commits too, because people type lists that way', async () => {
	const onchange = vi.fn();
	const screen = render(ChipInput, props({ onchange }));

	await screen.getByRole('textbox', { name: 'Add to tech stack' }).fill('SeaORM,');

	expect(onchange).toHaveBeenCalledWith(['Rust', 'SvelteKit', 'SeaORM']);
});

test('Backspace on an empty input takes the last one back', async () => {
	// §03 names this precisely. It only applies when the input is empty —
	// otherwise Backspace is editing the word being typed.
	const onchange = vi.fn();
	const screen = render(ChipInput, props({ onchange }));

	press(screen, 'Backspace');

	expect(onchange).toHaveBeenCalledWith(['Rust']);
});

test('Backspace while typing edits the word, not the list', async () => {
	const onchange = vi.fn();
	const screen = render(ChipInput, props({ onchange }));

	await screen.getByRole('textbox', { name: 'Add to tech stack' }).fill('Sea');
	press(screen, 'Backspace');

	expect(onchange).not.toHaveBeenCalled();
});

test('the same value is not added twice', async () => {
	const onchange = vi.fn();
	const screen = render(ChipInput, props({ onchange }));

	await screen.getByRole('textbox', { name: 'Add to tech stack' }).fill('Rust');
	press(screen, 'Enter');

	expect(onchange).not.toHaveBeenCalled();
});

test('whitespace is not a value', async () => {
	const onchange = vi.fn();
	const screen = render(ChipInput, props({ onchange }));

	await screen.getByRole('textbox', { name: 'Add to tech stack' }).fill('   ');
	press(screen, 'Enter');

	expect(onchange).not.toHaveBeenCalled();
});

test('says how it works, because a chip input is not obvious', async () => {
	const screen = render(ChipInput, props({ help: 'A chip input. Press Enter after each one.' }));

	await expect
		.element(screen.getByText('A chip input. Press Enter after each one.'))
		.toBeInTheDocument();
});

test('has no accessibility violations', async () => {
	render(ChipInput, props({ help: 'A chip input. Press Enter after each one.' }));

	await expectNoA11yViolations(document.body, UNSTYLED_GEOMETRY);
});
