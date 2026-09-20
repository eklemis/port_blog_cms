import { expect, test, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { expectNoA11yViolations } from '$lib/shared/test/a11y';
import ProjectsPage from './projects-page.svelte';

/** Screen / Projects list 70:2. */

/** These specs render without the stylesheet, so hit areas are not real. */
const UNSTYLED_GEOMETRY = { rules: { 'target-size': { enabled: false } } };

const ROWS = [
	{
		id: 'p-1',
		title: 'Blogport CMS',
		slug: 'blogport-cms',
		description: 'A portfolio CMS across four services.',
		tech_stack: ['Rust', 'SvelteKit'],
		topics: [{ id: 't-1', title: 'Rust' }],
		repo_url: 'https://github.com/eklemis/port_blog_cms',
		live_demo_url: 'https://blogport.example.test',
		updated_at: '2026-09-18T09:00:00Z'
	},
	{
		id: 'p-2',
		title: 'Heap visualiser',
		slug: 'heap-visualiser',
		description: null,
		tech_stack: ['TypeScript'],
		topics: [],
		repo_url: null,
		live_demo_url: 'https://heap.example.test',
		updated_at: '2026-07-02T09:00:00Z'
	}
];

const base = {
	projects: ROWS,
	topics: [{ id: 't-1', title: 'Rust' }],
	total: 2,
	everything: 2,
	page: 1,
	perPage: 10,
	filtered: false,
	onquery: () => {}
};

test('names the section and offers the one action that starts a project', async () => {
	const screen = render(ProjectsPage, base);

	await expect
		.element(screen.getByRole('heading', { level: 1, name: 'Projects' }))
		.toBeInTheDocument();
	await expect.element(screen.getByRole('link', { name: 'Add project' })).toBeInTheDocument();
});

test('says out loud that a project has no draft state', async () => {
	// 70:2 carries this above the table, and §02 is emphatic that it is said
	// rather than discovered: "Projects have no published_at. Creating one
	// publishes it."
	const screen = render(ProjectsPage, base);

	await expect
		.element(
			screen.getByText(
				'Projects have no draft state — adding one publishes it to your public page immediately.'
			)
		)
		.toBeInTheDocument();
});

test('each row leads to its editor', async () => {
	const screen = render(ProjectsPage, base);

	await expect
		.element(screen.getByRole('link', { name: 'Blogport CMS' }))
		.toHaveAttribute('href', '/studio/projects/p-1');
});

test('the columns are the ones the frame names', async () => {
	const screen = render(ProjectsPage, base);

	const headers = screen.container.querySelectorAll('thead th');

	expect([...headers].map((cell) => cell.textContent?.trim())).toEqual([
		'Project',
		'Stack',
		'Links',
		'Updated'
	]);
});

test('a project with one link shows one, not a stranded separator', async () => {
	const screen = render(ProjectsPage, base);

	const row = screen.getByRole('row', { name: /Heap visualiser/ });

	await expect.element(row.getByRole('link', { name: 'demo' })).toBeInTheDocument();
	expect(row.getByRole('link', { name: 'repo' }).elements()).toHaveLength(0);
});

test('searching asks for the search rather than filtering what is already here', async () => {
	// The list is a page of many, so a filter applied in the browser would hide
	// rows on this page and miss matches on every other.
	const onquery = vi.fn();
	const screen = render(ProjectsPage, { ...base, onquery });

	await screen.getByRole('searchbox', { name: 'Search projects' }).fill('rust');

	await vi.waitFor(() => expect(onquery).toHaveBeenCalled());
	expect(onquery.mock.calls.at(-1)?.[0]).toMatchObject({ search: 'rust', page: null });
});

test('choosing a topic filters the list and returns to the first page', async () => {
	// Page 3 of everything is not page 3 of one topic.
	const onquery = vi.fn();
	const screen = render(ProjectsPage, { ...base, onquery, page: 3 });

	await screen.getByRole('combobox', { name: 'Topic' }).selectOptions('t-1');

	expect(onquery).toHaveBeenCalledWith({ topic_id: 't-1', page: null });
});

test('an author with no projects is told how to start one', async () => {
	const screen = render(ProjectsPage, { ...base, projects: [], total: 0, everything: 0 });

	await expect.element(screen.getByText('No projects yet.')).toBeInTheDocument();

	// §04: "Why it's empty + the one action that fixes it." The header keeps its
	// own copy of that action, so the state is offering a second, nearer one.
	expect(screen.getByRole('link', { name: 'Add project' }).elements().length).toBeGreaterThan(1);
});

test('a filter that matches nothing is not the same as having nothing', async () => {
	// §04: conflating them tells someone with four projects that they have none.
	const screen = render(ProjectsPage, {
		...base,
		projects: [],
		total: 0,
		everything: 4,
		filtered: true,
		search: 'kafka'
	});

	await expect.element(screen.getByRole('button', { name: 'Clear filters' })).toBeInTheDocument();
	expect(screen.getByText('No projects yet.').elements()).toHaveLength(0);
});

test('a list that failed to load says so, and says the rest is safe', async () => {
	const screen = render(ProjectsPage, { ...base, projects: [], total: 0, failed: true });

	await expect.element(screen.getByRole('status')).toBeInTheDocument();
});

test('has no accessibility violations', async () => {
	render(ProjectsPage, base);

	await expectNoA11yViolations(document.body, UNSTYLED_GEOMETRY);
});
