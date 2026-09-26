import { expect, test, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { expectNoA11yViolations } from '$lib/shared/test/a11y';
import ExperienceList from './experience-list.svelte';

/**
 * The experience collection — Screen / CV builder 69:2.
 *
 * The frame states the rule under the list: "Rows collapse to a one-line
 * summary. **One opens at a time** — ten expanded rows is a wall, not a form."
 */

/** These specs render without the stylesheet, so hit areas are not real. */
const UNSTYLED_GEOMETRY = { rules: { 'target-size': { enabled: false } } };

const ROLES = [
	{
		company: 'Tokopedia',
		position: 'Backend Engineer',
		location: 'Jakarta',
		start_date: '2022-01',
		end_date: '2025-04',
		tasks: ['Built the order-events pipeline'],
		achievements: ['Reduced latency by 40%'],
		description: 'Led the platform team.'
	},
	{
		company: 'Bukalapak',
		position: 'Junior Engineer',
		location: 'Jakarta',
		start_date: '2020-02',
		end_date: '2022-01',
		tasks: [],
		achievements: [],
		description: ''
	}
];

const props = (over: Record<string, unknown> = {}) => ({
	roles: ROLES,
	onchange: () => {},
	...over
});

test('every role collapses to what it was and when', async () => {
	const screen = render(ExperienceList, props());

	await expect.element(screen.getByText('Backend Engineer · Tokopedia')).toBeInTheDocument();
	await expect.element(screen.getByText('Junior Engineer · Bukalapak')).toBeInTheDocument();
	await expect.element(screen.getByText('2020 – 2022')).toBeInTheDocument();
});

test('nothing is open until something is opened', async () => {
	const screen = render(ExperienceList, props());

	expect(screen.getByRole('textbox', { name: 'Company' }).elements()).toHaveLength(0);
});

test('opening a role shows the fields the frame draws', async () => {
	const screen = render(ExperienceList, props());

	await screen.getByRole('button', { name: 'Edit Backend Engineer · Tokopedia' }).click();

	await expect.element(screen.getByRole('textbox', { name: 'Company' })).toHaveValue('Tokopedia');
	await expect.element(screen.getByRole('textbox', { name: 'Location' })).toHaveValue('Jakarta');
	await expect.element(screen.getByRole('textbox', { name: 'Start' })).toHaveValue('2022-01');
});

test('one opens at a time, because ten open rows is a wall', async () => {
	// The frame says this in as many words under the list.
	const screen = render(ExperienceList, props());

	await screen.getByRole('button', { name: 'Edit Backend Engineer · Tokopedia' }).click();
	await screen.getByRole('button', { name: 'Edit Junior Engineer · Bukalapak' }).click();

	await expect.element(screen.getByRole('textbox', { name: 'Company' })).toHaveValue('Bukalapak');
	expect(screen.getByRole('textbox', { name: 'Company' }).elements()).toHaveLength(1);
});

test('editing a field reports the whole list back, not the field', async () => {
	// `experiences` is replaced wholesale, so the caller needs the list.
	const onchange = vi.fn();
	const screen = render(ExperienceList, props({ onchange }));

	await screen.getByRole('button', { name: 'Edit Backend Engineer · Tokopedia' }).click();
	await screen.getByRole('textbox', { name: 'Company' }).fill('Tokopedia Indonesia');

	await vi.waitFor(() => expect(onchange).toHaveBeenCalled());
	const next = onchange.mock.calls.at(-1)?.[0];
	expect(next).toHaveLength(2);
	expect(next[0].company).toBe('Tokopedia Indonesia');
});

test('a field the builder never draws is carried through untouched', async () => {
	// The list is replaced wholesale; `achievements` and `description` are on
	// the DTO and not on this form.
	const onchange = vi.fn();
	const screen = render(ExperienceList, props({ onchange }));

	await screen.getByRole('button', { name: 'Edit Backend Engineer · Tokopedia' }).click();
	await screen.getByRole('textbox', { name: 'Company' }).fill('Tokopedia Indonesia');

	await vi.waitFor(() => expect(onchange).toHaveBeenCalled());
	const next = onchange.mock.calls.at(-1)?.[0];
	expect(next[0].achievements).toEqual(['Reduced latency by 40%']);
	expect(next[0].description).toBe('Led the platform team.');
});

test('“I work here now” is the absence of an end date', async () => {
	const onchange = vi.fn();
	const screen = render(ExperienceList, props({ onchange }));

	await screen.getByRole('button', { name: 'Edit Backend Engineer · Tokopedia' }).click();
	await screen.getByRole('checkbox', { name: 'I work here now' }).click();

	await vi.waitFor(() => expect(onchange).toHaveBeenCalled());
	expect(onchange.mock.calls.at(-1)?.[0][0].end_date).toBe('');
});

test('a job someone still has offers no end date to type', async () => {
	// The field would be a box that cannot mean anything while the box above
	// it is ticked.
	const screen = render(ExperienceList, props());

	await screen.getByRole('button', { name: 'Edit Backend Engineer · Tokopedia' }).click();
	await screen.getByRole('checkbox', { name: 'I work here now' }).click();

	await expect.element(screen.getByRole('textbox', { name: 'End' })).toBeDisabled();
});

test('tasks can be added and taken off', async () => {
	const onchange = vi.fn();
	const screen = render(ExperienceList, props({ onchange }));

	await screen.getByRole('button', { name: 'Edit Backend Engineer · Tokopedia' }).click();
	await screen.getByRole('button', { name: 'Add a task' }).click();

	await vi.waitFor(() => expect(onchange).toHaveBeenCalled());
	expect(onchange.mock.calls.at(-1)?.[0][0].tasks).toHaveLength(2);
});

test('a new role opens straight away, because nobody adds one to leave it shut', async () => {
	const onchange = vi.fn();
	const screen = render(ExperienceList, props({ onchange }));

	await screen.getByRole('button', { name: 'Add role' }).click();

	await expect.element(screen.getByRole('textbox', { name: 'Company' })).toHaveValue('');
	expect(onchange.mock.calls.at(-1)?.[0]).toHaveLength(3);
});

test('a role can be removed while it is open', async () => {
	const onchange = vi.fn();
	const screen = render(ExperienceList, props({ onchange }));

	await screen.getByRole('button', { name: 'Edit Backend Engineer · Tokopedia' }).click();
	await screen.getByRole('button', { name: 'Remove Backend Engineer · Tokopedia' }).click();

	await vi.waitFor(() => expect(onchange).toHaveBeenCalled());
	expect(onchange.mock.calls.at(-1)?.[0]).toHaveLength(1);
});

test('has no accessibility violations', async () => {
	render(ExperienceList, props());

	await expectNoA11yViolations(document.body, UNSTYLED_GEOMETRY);
});
