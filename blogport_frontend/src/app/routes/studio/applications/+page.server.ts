import type { PageServerLoad } from './$types';
import { authenticatedFetch } from '$lib/shared/api/backend.server';
import { trackerRows, type Application, type Job } from '$lib/entities/application';

/**
 * `/studio/applications` — the tracker.
 *
 * Two lists, fetched together and joined by `job_id`: the applications carry
 * the status and the dates, the jobs carry the role and the company. Neither
 * endpoint takes a parameter, so there is no filtering, sorting or paging to
 * read off the URL — the map says as much, and §09 says the same thing from
 * the other direction when it explains why "no reply" cannot be a filter.
 *
 * The join is done here rather than in the page so the page renders rows and
 * nothing else.
 */

/** `null` when the list could not be had — which is not the same as empty. */
async function list<T>(event: Parameters<PageServerLoad>[0], path: string): Promise<T[] | null> {
	try {
		const response = await authenticatedFetch(event, path);
		if (!response.ok) return null;

		const body = (await response.json()) as { data?: unknown };
		return Array.isArray(body.data) ? (body.data as T[]) : [];
	} catch {
		return null;
	}
}

export const load: PageServerLoad = async (event) => {
	const [applications, jobs] = await Promise.all([
		list<Application>(event, '/api/applications'),
		list<Job>(event, '/api/jobs')
	]);

	// Losing the jobs is a worse row, not an error screen: the applications are
	// real either way, and a row without its role still says a status and a date.
	if (!applications) return { rows: [], failed: true };

	return { rows: trackerRows(applications, jobs ?? []), failed: false };
};
