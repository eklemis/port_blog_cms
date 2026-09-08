import { beforeEach, expect, test, vi } from 'vitest';

const fetchImpl = vi.fn();

/**
 * Set to make the call throw. A plain variable rather than a rejecting mock:
 * Vitest tracks a `vi.fn`'s settled result too, and that second promise is
 * reported as unhandled even when the loader catches the first.
 */
let unreachable: Error | null = null;

vi.mock('$lib/shared/api/backend.server', () => ({
	authenticatedFetch: async (...args: unknown[]) => {
		if (unreachable) throw unreachable;
		return fetchImpl(...args);
	}
}));

const { load } = await import('./+page.server');

/**
 * The Overview's counts.
 *
 * Four headline numbers and the one the first-run checklist needs. The three
 * paginated resources answer with a `total` at `per_page=1`; applications and
 * topics are unpaginated and answer with the whole list, so their count is its
 * length.
 */

/** Answers by path, since all five calls go out together. */
function backend(bodies: Record<string, { ok?: boolean; body: unknown }>) {
	fetchImpl.mockImplementation(async (_event: unknown, path: string) => {
		const key = Object.keys(bodies).find((candidate) => String(path).startsWith(candidate));
		if (!key) throw new Error(`unexpected path ${path}`);
		return { ok: bodies[key].ok ?? true, json: async () => bodies[key].body };
	});
}

const paged = (total: number) => ({ body: { data: { total } } });
const listed = (length: number) => ({ body: { data: Array.from({ length }, () => ({})) } });

const ALL = {
	'/api/blog': paged(24),
	'/api/projects': paged(8),
	'/api/cvs': paged(3),
	'/api/applications': listed(5),
	'/api/topics': listed(6)
};

async function loaded() {
	const data = await load({} as never);
	return data as unknown as {
		counts: Record<'posts' | 'projects' | 'resumes' | 'applications' | 'topics', number | null>;
	};
}

beforeEach(() => {
	fetchImpl.mockReset();
	unreachable = null;
});

test('counts every resource the map lists for this screen', async () => {
	backend(ALL);

	const { counts } = await loaded();

	expect(counts).toEqual({ posts: 24, projects: 8, resumes: 3, applications: 5, topics: 6 });
});

test('asks the paginated resources for one row, not for all of them', async () => {
	backend(ALL);

	await loaded();

	const asked = fetchImpl.mock.calls.map((call) => String(call[1]));
	expect(asked).toContain('/api/blog?per_page=1');
	// Applications and topics take no parameters at all, so asking for a page
	// of one would be a query string the backend quietly ignores.
	expect(asked).toContain('/api/applications');
});

test('a count that could not be had is null, never zero', async () => {
	// A tile showing 0 sends someone looking for work that has not gone
	// anywhere; an em dash says we could not ask.
	backend({ ...ALL, '/api/blog': { ok: false, body: {} } });

	const { counts } = await loaded();

	expect(counts.posts).toBeNull();
	expect(counts.projects).toBe(8);
});

test('an unreachable backend leaves every count unknown rather than throwing', async () => {
	unreachable = new Error('offline');

	const { counts } = await loaded();

	expect(counts).toEqual({
		posts: null,
		projects: null,
		resumes: null,
		applications: null,
		topics: null
	});
});

test('a body in a shape we did not expect is unknown, not a guess', async () => {
	backend({ ...ALL, '/api/topics': { body: { data: 'not a list' } } });

	const { counts } = await loaded();

	expect(counts.topics).toBeNull();
});
