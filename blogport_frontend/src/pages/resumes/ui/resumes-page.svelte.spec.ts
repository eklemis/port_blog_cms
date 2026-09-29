import { expect, test, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { expectNoA11yViolations } from '$lib/shared/test/a11y';
import ResumesPage from './resumes-page.svelte';

/**
 * `/studio/resumes` — Screen / Résumés 241:5528.
 *
 * Three columns: Résumé, Summary, Sections. No Updated column, which is the
 * one thing about this screen worth knowing: `CvResponse` carries no
 * `updated_at` while `CVSort` offers `updated_newest`, so the list can be
 * sorted by a date it cannot show.
 */

/** These specs render without the stylesheet, so hit areas are not real. */
const UNSTYLED_GEOMETRY = { rules: { 'target-size': { enabled: false } } };

const ROWS = [
	{
		id: 'cv-1',
		role: 'Senior Backend',
		bio: 'Backend engineer, mostly Rust. Ten years on payments and logistics.',
		experiences: [{}, {}, {}, {}, {}, {}],
		highlighted_projects: [{}, {}, {}]
	},
	{
		id: 'cv-2',
		role: 'Platform Engineer',
		bio: 'Infrastructure and developer tooling, with a bias toward boring.',
		experiences: [{}, {}, {}, {}, {}, {}],
		highlighted_projects: [{}, {}]
	}
];

const json = (body: unknown, status = 200) => Response.json(body, { status });

const props = (over: Record<string, unknown> = {}) => ({
	resumes: ROWS,
	total: ROWS.length,
	page: 1,
	perPage: 10,
	filtered: false,
	displayName: 'Ada Lovelace',
	onquery: () => {},
	oncreated: () => {},
	fetchFn: vi.fn(async () => json({ data: { id: 'cv-new' } }, 201)) as unknown as typeof fetch,
	...over
});

test('names each résumé by its role, and says what is on it', async () => {
	const screen = render(ResumesPage, props());

	await expect.element(screen.getByRole('columnheader', { name: 'Résumé' })).toBeInTheDocument();
	await expect.element(screen.getByRole('columnheader', { name: 'Sections' })).toBeInTheDocument();

	await expect.element(screen.getByText('Senior Backend')).toBeInTheDocument();
	await expect
		.element(
			screen.getByText('Backend engineer, mostly Rust. Ten years on payments and logistics.')
		)
		.toBeInTheDocument();
	await expect.element(screen.getByText('6 roles · 3 projects')).toBeInTheDocument();
});

test('a row opens its builder', async () => {
	const screen = render(ResumesPage, props());

	await expect
		.element(screen.getByRole('link', { name: 'Senior Backend' }))
		.toHaveAttribute('href', '/studio/resumes/cv-1');
});

test('a résumé with no role yet is still findable in the list', async () => {
	// It is born without one — J6 names it in the builder, not before. A row
	// rendering as a blank cell is a row nobody can click.
	const screen = render(
		ResumesPage,
		props({ resumes: [{ ...ROWS[0], role: '', bio: '' }], total: 1 })
	);

	await expect
		.element(screen.getByRole('link', { name: 'Untitled résumé' }))
		.toHaveAttribute('href', '/studio/resumes/cv-1');
});

test('says why there is more than one of these, in the frame’s words', async () => {
	const screen = render(ResumesPage, props());

	await expect
		.element(
			screen.getByText(
				'Several résumés is the point — one tailored per audience. Name each by the role it is for.'
			)
		)
		.toBeInTheDocument();
});

test('searching goes to the server, once the typing stops', async () => {
	vi.useFakeTimers();
	const queries: Record<string, string | null>[] = [];
	const screen = render(
		ResumesPage,
		props({ onquery: (q: Record<string, string | null>) => queries.push(q) })
	);

	await screen.getByRole('searchbox', { name: 'Search résumés' }).fill('backend');
	vi.advanceTimersByTime(300);
	vi.useRealTimers();

	// A page of ten filtered in the browser hides rows here and misses every
	// match on the other pages.
	expect(queries).toEqual([{ search: 'backend', page: null }]);
});

test('sorting goes to the server too, and starts the paging over', async () => {
	const queries: Record<string, string | null>[] = [];
	const screen = render(
		ResumesPage,
		props({ onquery: (q: Record<string, string | null>) => queries.push(q) })
	);

	await screen.getByRole('combobox', { name: 'Sort' }).selectOptions('newest');

	expect(queries).toEqual([{ sort: 'newest', page: null }]);
});

test('New résumé makes the empty document and opens it', async () => {
	const opened: string[] = [];
	const fetchFn = vi.fn(async () => json({ data: { id: 'cv-new' } }, 201));
	const screen = render(
		ResumesPage,
		props({
			fetchFn: fetchFn as unknown as typeof fetch,
			oncreated: (id: string) => opened.push(id)
		})
	);

	await screen.getByRole('button', { name: 'New résumé' }).click();
	await vi.waitFor(() => expect(opened).toEqual(['cv-new']));

	// The name is the one thing known before the builder opens.
	const [, init] = fetchFn.mock.calls[0] as unknown as [string, RequestInit];
	expect(JSON.parse(String(init.body))).toMatchObject({ display_name: 'Ada Lovelace' });
});

test('a refused create says so and opens nothing', async () => {
	const opened: string[] = [];
	const screen = render(
		ResumesPage,
		props({
			fetchFn: vi.fn(async () => json({ error: { code: 'VALIDATION_ERROR' } }, 422)),
			oncreated: (id: string) => opened.push(id)
		})
	);

	await screen.getByRole('button', { name: 'New résumé' }).click();

	// role="status", never role="alert": §09 reserves assertive for loss, and a
	// refused create has lost nothing.
	await expect.element(screen.getByText('Something went wrong on our side.')).toBeInTheDocument();
	expect(opened, 'nothing was opened').toEqual([]);
});

test('no résumés yet says what one is for, not that the list is empty', async () => {
	const screen = render(ResumesPage, props({ resumes: [], total: 0 }));

	await expect.element(screen.getByText('No résumés yet.')).toBeInTheDocument();
	await expect
		.element(
			screen.getByText(
				'One document per audience. This one starts empty and is named as you write it.'
			)
		)
		.toBeInTheDocument();
});

test('a search that matches nothing is told apart from having nothing', async () => {
	const screen = render(
		ResumesPage,
		props({ resumes: [], total: 0, filtered: true, everything: 4, search: 'rust' })
	);

	await expect.element(screen.getByText('No résumés match that.')).toBeInTheDocument();
	await expect.element(screen.getByText(/You have 4 résumés/)).toBeInTheDocument();
});

test('a list that did not load says the writing is safe', async () => {
	const screen = render(ResumesPage, props({ resumes: [], total: 0, failed: true }));

	await expect
		.element(screen.getByText('That list didn’t load. Nothing you’ve written is affected.'))
		.toBeInTheDocument();
});

test('the screen has no accessibility violations', async () => {
	render(ResumesPage, props());

	await expectNoA11yViolations(document.body, UNSTYLED_GEOMETRY);
});
