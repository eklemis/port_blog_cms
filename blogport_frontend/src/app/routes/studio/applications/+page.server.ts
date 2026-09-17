import type { PageServerLoad } from './$types';
import { authenticatedFetch } from '$lib/shared/api/backend.server';
import { trackerRows, type Application, type Job } from '$lib/entities/application';

/**
 * `/studio/applications` — the tracker.
 *
 * Two lists joined by `job_id`: the applications carry the status and the
 * dates, the jobs carry the role and the company. The join is done here rather
 * than in the page, so the page renders rows and nothing else.
 *
 * Both listings page now. They did not when this screen was written, and they
 * answer with the backend's own default of ten when asked for nothing — so a
 * tracker that sent no parameters would quietly show one page and call it the
 * whole list.
 *
 * The jobs call asks for the ids this page actually names. It used to ask for
 * the largest page the API allows and hope the right ones were in it, so past a
 * hundred jobs some rows silently lost their role and company. Asking by id
 * costs a second round trip — the ids are not known until the applications
 * arrive — and buys a join that is right rather than likely.
 */

/** Blueprint §06: the default every list shares. */
const PER_PAGE = 10;

type Paged<T> = { items?: T[]; total?: number; page?: number; per_page?: number };

/** `null` when the list could not be had — which is not the same as empty. */
async function page<T>(
	event: Parameters<PageServerLoad>[0],
	path: string
): Promise<Paged<T> | null> {
	try {
		const response = await authenticatedFetch(event, path);
		if (!response.ok) return null;

		const body = (await response.json()) as { data?: unknown };
		const data = body.data as Paged<T> | undefined;

		return Array.isArray(data?.items) ? data : {};
	} catch {
		return null;
	}
}

function pageFrom(raw: string | null): number {
	const value = Number(raw);
	return Number.isInteger(value) && value > 0 ? value : 1;
}

export const load: PageServerLoad = async (event) => {
	const wanted = pageFrom(event.url.searchParams.get('page'));

	const query = new URLSearchParams({ page: String(wanted), per_page: String(PER_PAGE) });
	const applications = await page<Application>(event, `/api/applications?${query}`);

	const empty = { rows: [], total: 0, page: wanted, perPage: PER_PAGE };

	// Losing the jobs is a worse row, not an error screen: the applications are
	// real either way, and a row without its role still says a status and a date.
	if (!applications) return { ...empty, failed: true };

	// One entry per job, however many rows name it.
	const ids = [...new Set((applications.items ?? []).map((application) => application.job_id))];

	// No rows name no jobs, and a request for none of them is worth not making.
	const jobs = ids.length
		? await page<Job>(
				event,
				`/api/jobs?${new URLSearchParams({ ids: ids.join(','), per_page: String(ids.length) })}`
			)
		: { items: [] };

	return {
		rows: trackerRows(applications.items ?? [], jobs?.items ?? []),
		total: applications.total ?? 0,
		page: applications.page ?? wanted,
		perPage: applications.per_page ?? PER_PAGE,
		failed: false
	};
};
