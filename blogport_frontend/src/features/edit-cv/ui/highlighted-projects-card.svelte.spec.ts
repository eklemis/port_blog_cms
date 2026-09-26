import { expect, test, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { expectNoA11yViolations } from '$lib/shared/test/a11y';
import HighlightedProjectsCard from './highlighted-projects-card.svelte';

/**
 * Highlighted projects — Screen / CV builder 69:2, the fourth rail card.
 *
 * `HighlightedProjectDto` carries an `id` and a `slug`, so a row is **chosen**
 * from the author's own projects rather than typed. That is what makes this
 * card different from the other three: adding is a pick, and the identity of
 * the row is the project's rather than the résumé's.
 */

/** These specs render without the stylesheet, so hit areas are not real. */
const UNSTYLED_GEOMETRY = { rules: { 'target-size': { enabled: false } } };

const PROJECTS = [
	{ id: 'p-1', title: 'Blogport CMS', slug: 'blogport-cms' },
	{ id: 'p-2', title: 'Heap visualiser', slug: 'heap-visualiser' },
	{ id: 'p-3', title: 'Order pipeline', slug: 'order-pipeline' }
];

const CHOSEN = [
	{ id: 'p-1', title: 'Blogport CMS', slug: 'blogport-cms', short_description: 'A portfolio CMS.' }
];

const props = (over: Record<string, unknown> = {}) => ({
	chosen: CHOSEN,
	projects: PROJECTS,
	onchange: () => {},
	...over
});

test('lists what the résumé already highlights', async () => {
	const screen = render(HighlightedProjectsCard, props());

	await expect.element(screen.getByText('Blogport CMS')).toBeInTheDocument();
});

test('offers the projects that are not on it yet', async () => {
	// Offering one already chosen would be a way to add it twice.
	const screen = render(HighlightedProjectsCard, props());

	const picker = screen.getByRole('combobox', { name: 'Project to highlight' });

	await expect.element(picker.getByRole('option', { name: 'Heap visualiser' })).toBeInTheDocument();
	expect(picker.getByRole('option', { name: 'Blogport CMS' }).elements()).toHaveLength(0);
});

test('choosing one takes its identity from the project, not from typing', async () => {
	// The id and slug are the project's. A résumé that invented them would
	// point at nothing.
	const onchange = vi.fn();
	const screen = render(HighlightedProjectsCard, props({ onchange }));

	await screen.getByRole('combobox', { name: 'Project to highlight' }).selectOptions('p-2');
	await screen.getByRole('button', { name: 'Add project' }).click();

	await vi.waitFor(() => expect(onchange).toHaveBeenCalled());
	const next = onchange.mock.calls.at(-1)?.[0];
	expect(next).toHaveLength(2);
	expect(next[1]).toMatchObject({ id: 'p-2', slug: 'heap-visualiser', title: 'Heap visualiser' });
});

test('a newly chosen project starts with no summary, because that line is the résumé’s', async () => {
	// The project's own description is its description. What appears under a
	// role on a CV is written for the CV.
	const onchange = vi.fn();
	const screen = render(HighlightedProjectsCard, props({ onchange }));

	await screen.getByRole('combobox', { name: 'Project to highlight' }).selectOptions('p-2');
	await screen.getByRole('button', { name: 'Add project' }).click();

	await vi.waitFor(() => expect(onchange).toHaveBeenCalled());
	expect(onchange.mock.calls.at(-1)?.[0][1].short_description).toBe('');
});

test('nothing is added until a project is chosen', async () => {
	const onchange = vi.fn();
	const screen = render(HighlightedProjectsCard, props({ onchange }));

	await expect.element(screen.getByRole('button', { name: 'Add project' })).toBeDisabled();
	expect(onchange).not.toHaveBeenCalled();
});

test('the summary is the one thing about a row that can be edited here', async () => {
	const onchange = vi.fn();
	const screen = render(HighlightedProjectsCard, props({ onchange }));

	await screen.getByRole('button', { name: 'Edit Blogport CMS' }).click();
	await screen.getByRole('textbox', { name: 'One-line summary' }).fill('Four services, one CMS.');

	await vi.waitFor(() => expect(onchange).toHaveBeenCalled());
	const next = onchange.mock.calls.at(-1)?.[0];
	expect(next[0].short_description).toBe('Four services, one CMS.');
	expect(next[0].id).toBe('p-1');
});

test('a project can be taken off the résumé', async () => {
	const onchange = vi.fn();
	const screen = render(HighlightedProjectsCard, props({ onchange }));

	await screen.getByRole('button', { name: 'Edit Blogport CMS' }).click();
	await screen.getByRole('button', { name: 'Remove Blogport CMS' }).click();

	await vi.waitFor(() => expect(onchange).toHaveBeenCalled());
	expect(onchange.mock.calls.at(-1)?.[0]).toHaveLength(0);
});

test('an author with every project already highlighted is offered none', async () => {
	const screen = render(
		HighlightedProjectsCard,
		props({ projects: [PROJECTS[0]], chosen: CHOSEN })
	);

	await expect
		.element(screen.getByText('Every project is already on this résumé.'))
		.toBeInTheDocument();
});

test('an author with no projects is told where projects come from', async () => {
	const screen = render(HighlightedProjectsCard, props({ projects: [], chosen: [] }));

	await expect
		.element(screen.getByText('No projects yet — add one under Projects first.'))
		.toBeInTheDocument();
});

test('has no accessibility violations', async () => {
	render(HighlightedProjectsCard, props());

	await expectNoA11yViolations(document.body, UNSTYLED_GEOMETRY);
});
