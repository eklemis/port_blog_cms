import { beforeEach, expect, test, vi } from 'vitest';
import type { TrackerRow } from '$lib/entities/application';

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
 * The application tracker.
 *
 * Two unparameterised lists joined by `job_id` — the map's own note, and the
 * only way to put a role and a company on a row without one request per row.
 */

const APPLICATIONS = {
	data: [
		{
			id: 'app-1',
			job_id: 'job-1',
			status: 'interview',
			next_action: 'Send the take-home',
			applied_at: '2026-09-06T12:00:00Z',
			created_at: '2026-09-01T12:00:00Z',
			updated_at: '2026-09-06T12:00:00Z'
		}
	]
};

const JOBS = { data: [{ id: 'job-1', title: 'Senior Backend', company: 'Gojek' }] };

/** Answers by path, because the two calls go out together. */
function backend(bodies: Record<string, { ok?: boolean; body: unknown }>) {
	fetchImpl.mockImplementation(async (_event: unknown, path: string) => {
		const key = Object.keys(bodies).find((candidate) => String(path).startsWith(candidate));
		if (!key) throw new Error(`unexpected path ${path}`);
		return { ok: bodies[key].ok ?? true, json: async () => bodies[key].body };
	});
}

/**
 * The loader's return, narrowed. SvelteKit's generated `load` type is a union
 * with `void`, so the properties are only reachable through a cast — the posts
 * spec sidesteps the same thing with `toMatchObject`.
 */
async function loaded() {
	const data = await load({ url: new URL('http://localhost/studio/applications') } as never);
	return data as unknown as { rows: TrackerRow[]; failed: boolean };
}

beforeEach(() => {
	fetchImpl.mockReset();
	unreachable = null;
});

test('asks for both lists, and asks for them at the same time', async () => {
	backend({ '/api/applications': { body: APPLICATIONS }, '/api/jobs': { body: JOBS } });

	await loaded();

	const asked = fetchImpl.mock.calls.map((call) => String(call[1]));
	expect(asked).toContain('/api/applications');
	expect(asked).toContain('/api/jobs');
});

test('hands through rows already joined, so the page does no lookups', async () => {
	backend({ '/api/applications': { body: APPLICATIONS }, '/api/jobs': { body: JOBS } });

	const data = await loaded();

	expect(data.rows).toHaveLength(1);
	expect(data.rows[0].role).toBe('Senior Backend');
	expect(data.rows[0].company).toBe('Gojek');
	expect(data.failed).toBe(false);
});

test('applications failing is the failure — there is nothing to show', async () => {
	backend({
		'/api/applications': { ok: false, body: {} },
		'/api/jobs': { body: JOBS }
	});

	const data = await loaded();

	expect(data.failed).toBe(true);
	expect(data.rows).toEqual([]);
});

test('jobs failing is not: the applications are still real', async () => {
	// Losing the role and the company is a worse row, not an error screen.
	backend({
		'/api/applications': { body: APPLICATIONS },
		'/api/jobs': { ok: false, body: {} }
	});

	const data = await loaded();

	expect(data.failed).toBe(false);
	expect(data.rows).toHaveLength(1);
	expect(data.rows[0].role).toBe('Untitled role');
});

test('an unreachable backend is the error state, not a crash', async () => {
	unreachable = new Error('offline');

	const data = await loaded();

	expect(data.failed).toBe(true);
	expect(data.rows).toEqual([]);
});

test('a body in a shape we did not expect is empty, not undefined', async () => {
	backend({ '/api/applications': { body: {} }, '/api/jobs': { body: {} } });

	const data = await loaded();

	expect(data.rows).toEqual([]);
	expect(data.failed).toBe(false);
});
