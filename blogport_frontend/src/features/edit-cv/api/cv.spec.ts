import { expect, test, vi } from 'vitest';
import { patchCv, replaceExperiences } from './cv';

/**
 * Saving a résumé.
 *
 * `PATCH /api/cvs/{id}` is field-level, and each collection is a `ReplaceOp`:
 * "The full replacement list. There is no per-item patch: a list is replaced
 * wholesale or left alone."
 *
 * That wholesale replacement is the trap this file exists to avoid.
 */

const ok = (body: unknown, status = 200) =>
	new Response(JSON.stringify(body), { status, headers: { 'content-type': 'application/json' } });

const sent = (mock: { mock: { calls: unknown[] } }) =>
	mock.mock.calls as unknown as [string, RequestInit][];

const role = (over: Record<string, unknown> = {}) => ({
	company: 'Tokopedia',
	position: 'Backend Engineer',
	location: 'Jakarta',
	start_date: '2022-01',
	end_date: '2025-04',
	tasks: ['Built the order-events pipeline'],
	achievements: ['Reduced latency by 40%'],
	description: 'Led the platform team.',
	...over
});

test('sends only the parts that were edited', async () => {
	const fetchFn = vi.fn(async () => ok({ data: { id: 'cv-1' } }));

	await patchCv('cv-1', { role: 'Staff Engineer' }, fetchFn as unknown as typeof fetch);

	const [url, init] = sent(fetchFn)[0];
	expect(url).toBe('/api/cvs/cv-1');
	expect(init.method).toBe('PATCH');
	expect(JSON.parse(String(init.body))).toEqual({ role: 'Staff Engineer' });
});

test('a list is wrapped in the replace op the endpoint expects', async () => {
	const fetchFn = vi.fn(async () => ok({ data: { id: 'cv-1' } }));

	await patchCv(
		'cv-1',
		{ experiences: replaceExperiences([role()]) },
		fetchFn as unknown as typeof fetch
	);

	const body = JSON.parse(String(sent(fetchFn)[0][1].body));
	expect(body.experiences).toHaveProperty('replace');
	expect(body.experiences.replace).toHaveLength(1);
});

test('fields the builder never draws survive a save', async () => {
	// The frame draws company, position, location, dates and tasks. The DTO
	// also requires `achievements` and `description`, and the list is replaced
	// wholesale — so rebuilding rows from the drawn fields alone would delete
	// work nobody touched.
	const fetchFn = vi.fn(async () => ok({ data: { id: 'cv-1' } }));

	await patchCv(
		'cv-1',
		{ experiences: replaceExperiences([role()]) },
		fetchFn as unknown as typeof fetch
	);

	const [first] = JSON.parse(String(sent(fetchFn)[0][1].body)).experiences.replace;
	expect(first.achievements).toEqual(['Reduced latency by 40%']);
	expect(first.description).toBe('Led the platform team.');
});

test('a role someone still holds sends no end date at all', async () => {
	// "Absent for a current position" — an empty string is a value, and would
	// read as an end date that is blank rather than a job still running.
	const fetchFn = vi.fn(async () => ok({ data: { id: 'cv-1' } }));

	await patchCv(
		'cv-1',
		{ experiences: replaceExperiences([role({ end_date: '' })]) },
		fetchFn as unknown as typeof fetch
	);

	const [first] = JSON.parse(String(sent(fetchFn)[0][1].body)).experiences.replace;
	expect(first).not.toHaveProperty('end_date');
});

test('empty tasks are dropped rather than saved as blank lines', async () => {
	const fetchFn = vi.fn(async () => ok({ data: { id: 'cv-1' } }));

	await patchCv(
		'cv-1',
		{ experiences: replaceExperiences([role({ tasks: ['Real work', '   ', ''] })]) },
		fetchFn as unknown as typeof fetch
	);

	const [first] = JSON.parse(String(sent(fetchFn)[0][1].body)).experiences.replace;
	expect(first.tasks).toEqual(['Real work']);
});

test('a refusal is reported rather than looking saved', async () => {
	const fetchFn = vi.fn(async () => ok({ error: { code: 'CV_UNAUTHORIZED' } }, 403));

	await expect(
		patchCv('cv-1', { role: 'x' }, fetchFn as unknown as typeof fetch)
	).resolves.toMatchObject({ ok: false, kind: 'gate' });
});

test('an unreachable server is not blamed on the person', async () => {
	const fetchFn = vi.fn(async () => {
		throw new Error('offline');
	});

	await expect(
		patchCv('cv-1', { role: 'x' }, fetchFn as unknown as typeof fetch)
	).resolves.toMatchObject({ ok: false, kind: 'notOurs' });
});
