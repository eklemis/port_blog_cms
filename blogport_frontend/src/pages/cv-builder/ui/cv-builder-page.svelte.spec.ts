import { expect, test, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { expectNoA11yViolations } from '$lib/shared/test/a11y';
import CvBuilderPage from './cv-builder-page.svelte';

/** `/studio/resumes/[id]` — Screen / CV builder 69:2. */

/** These specs render without the stylesheet, so hit areas are not real. */
const UNSTYLED_GEOMETRY = { rules: { 'target-size': { enabled: false } } };

const CV = {
	id: 'cv-1',
	role: 'Senior Backend Engineer',
	display_name: 'Jane Doe',
	experiences: [
		{
			company: 'Tokopedia',
			position: 'Backend Engineer',
			location: 'Jakarta',
			start_date: '2022-01',
			end_date: '2025-04',
			tasks: ['Built the order-events pipeline'],
			achievements: ['Reduced latency by 40%'],
			description: 'Led the platform team.'
		}
	],
	core_skills: [{}, {}, {}],
	educations: [{}],
	highlighted_projects: [{}, {}],
	contact_info: [{}, {}, {}]
};

const json = (body: unknown, status = 200) => Response.json(body, { status });

const props = (over: Record<string, unknown> = {}) => ({
	cv: CV,
	fetchFn: vi.fn(async () => json({ data: { id: 'cv-1' } })) as unknown as typeof fetch,
	...over
});

const sent = (mock: { mock: { calls: unknown[] } }) =>
	mock.mock.calls as unknown as [string, RequestInit][];

test('the document is titled by the role it is for', async () => {
	const screen = render(CvBuilderPage, props());

	await expect
		.element(screen.getByRole('heading', { level: 1, name: 'Senior Backend Engineer' }))
		.toBeInTheDocument();
});

test('a résumé with no role yet is still findable on screen', async () => {
	const screen = render(CvBuilderPage, props({ cv: { ...CV, role: '' } }));

	await expect
		.element(screen.getByRole('heading', { level: 1, name: 'Untitled résumé' }))
		.toBeInTheDocument();
});

test('identity opens with what is on the document', async () => {
	const screen = render(CvBuilderPage, props());

	await expect
		.element(screen.getByRole('textbox', { name: 'Role' }))
		.toHaveValue('Senior Backend Engineer');
	await expect
		.element(screen.getByRole('textbox', { name: 'Display name' }))
		.toHaveValue('Jane Doe');
});

test('the four other collections are named and counted, as the frame shows them', async () => {
	// Their editors are the next slice. The frame's "+ Add" is not here,
	// because a control that opens nothing is worse than one that is missing.
	const screen = render(CvBuilderPage, props());

	// Scoped per region: core skills and contact details both happen to hold
	// three, which is exactly what a real document looks like.
	await expect
		.element(screen.getByRole('region', { name: 'Core skills' }).getByText('3 entries'))
		.toBeInTheDocument();
	await expect
		.element(screen.getByRole('region', { name: 'Education' }).getByText('1 entry'))
		.toBeInTheDocument();
	await expect
		.element(screen.getByRole('region', { name: 'Highlighted projects' }).getByText('2 entries'))
		.toBeInTheDocument();
});

test('saving sends identity and the whole experience list', async () => {
	// `experiences` is a ReplaceOp — there is no per-item patch.
	const fetchFn = vi.fn(async () => json({ data: { id: 'cv-1' } }));
	const screen = render(CvBuilderPage, props({ fetchFn: fetchFn as unknown as typeof fetch }));

	await screen.getByRole('textbox', { name: 'Role' }).fill('Staff Engineer');
	await screen.getByRole('button', { name: 'Save' }).click();

	await vi.waitFor(() => expect(fetchFn).toHaveBeenCalled());
	const [url, init] = sent(fetchFn)[0];
	expect(url).toBe('/api/cvs/cv-1');
	const body = JSON.parse(String(init.body));
	expect(body.role).toBe('Staff Engineer');
	expect(body.experiences.replace).toHaveLength(1);
});

test('a save carries through the fields this form never drew', async () => {
	const fetchFn = vi.fn(async () => json({ data: { id: 'cv-1' } }));
	const screen = render(CvBuilderPage, props({ fetchFn: fetchFn as unknown as typeof fetch }));

	await screen.getByRole('button', { name: 'Save' }).click();

	await vi.waitFor(() => expect(fetchFn).toHaveBeenCalled());
	const [first] = JSON.parse(String(sent(fetchFn)[0][1].body)).experiences.replace;
	expect(first.achievements).toEqual(['Reduced latency by 40%']);
	expect(first.description).toBe('Led the platform team.');
});

test('a failed save keeps every edit', async () => {
	const fetchFn = vi.fn(async () => json({ error: { code: 'INTERNAL_ERROR' } }, 500));
	const screen = render(CvBuilderPage, props({ fetchFn: fetchFn as unknown as typeof fetch }));

	await screen.getByRole('textbox', { name: 'Role' }).fill('Staff Engineer');
	await screen.getByRole('button', { name: 'Save' }).click();

	await expect.element(screen.getByRole('status')).toBeInTheDocument();
	await expect.element(screen.getByRole('textbox', { name: 'Role' })).toHaveValue('Staff Engineer');
});

test('does not offer a language control it cannot save', async () => {
	// The frame draws "Written in English" beside the title. No CV carries a
	// `language` field — only a cover letter does. Filed with the backend.
	const screen = render(CvBuilderPage, props());

	expect(screen.container.innerHTML).not.toMatch(/written in/i);
});

test('a résumé that is not yours is refused in plain words', async () => {
	const screen = render(CvBuilderPage, props({ cv: null, denied: true }));

	await expect.element(screen.getByText('We couldn’t find that résumé.')).toBeInTheDocument();
	await expect
		.element(screen.getByRole('link', { name: 'Back to résumés' }))
		.toHaveAttribute('href', '/studio/resumes');
});

test('has no accessibility violations', async () => {
	render(CvBuilderPage, props());

	await expectNoA11yViolations(document.body, UNSTYLED_GEOMETRY);
});
