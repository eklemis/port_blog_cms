import { expect, test, vi } from 'vitest';
import { createCv } from './create-cv';

/**
 * "+ New résumé" — Screen / Résumés 241:5528, and J6's first step:
 * "POST /api/cvs → 201 { id }", then "Start from role and identity" in the
 * builder.
 *
 * There is no create-a-résumé screen, so this makes the empty document and the
 * builder fills it. `CreateCVRequest` requires every field including the five
 * collections, so the whole shape goes even though all of it is empty.
 */

const answering = (status: number, body: unknown) =>
	vi.fn(async () => new Response(JSON.stringify(body), { status }));

test('sends every field the create request requires, empty', async () => {
	const fetchFn = answering(201, { data: { id: 'cv-1' } });

	await createCv('Ada Lovelace', fetchFn as unknown as typeof fetch);

	const [url, init] = fetchFn.mock.calls[0] as unknown as [string, RequestInit];
	expect(url).toBe('/api/cvs');
	expect(init.method).toBe('POST');

	// Not a subset: the endpoint requires all nine, and a missing one is a 422
	// on a button that has nothing to say about it.
	expect(JSON.parse(String(init.body))).toEqual({
		bio: '',
		contact_info: [],
		core_skills: [],
		display_name: 'Ada Lovelace',
		educations: [],
		experiences: [],
		highlighted_projects: [],
		photo_url: '',
		role: ''
	});
});

test('hands back the id the builder is opened at', async () => {
	const result = await createCv(
		'Ada Lovelace',
		answering(201, { data: { id: 'cv-7' } }) as unknown as typeof fetch
	);

	expect(result).toEqual({ ok: true, id: 'cv-7' });
});

test('a 201 with no id is a failure, not a navigation to nowhere', async () => {
	// A `/studio/resumes/undefined` is a 404 blamed on the résumé rather than on
	// the response that had no id in it.
	const result = await createCv(
		'Ada Lovelace',
		answering(201, { data: {} }) as unknown as typeof fetch
	);

	expect(result.ok).toBe(false);
});

test('a refusal is classified, so the page knows what recovery to offer', async () => {
	const result = await createCv(
		'Ada Lovelace',
		answering(403, { error: { code: 'EMAIL_NOT_VERIFIED' } }) as unknown as typeof fetch
	);

	expect(result).toMatchObject({ ok: false, kind: 'gate' });
});

test('an unreachable network is a failure the page can say out loud', async () => {
	const result = await createCv(
		'Ada Lovelace',
		vi.fn(async () => {
			throw new TypeError('offline');
		}) as unknown as typeof fetch
	);

	expect(result).toMatchObject({ ok: false, kind: 'notOurs' });
});
