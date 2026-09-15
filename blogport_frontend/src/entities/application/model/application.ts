import type { components } from '$lib/shared/api/v1';

/**
 * What one row of the application tracker can say about itself.
 *
 * `ApplicationResponse` carries a status, two dates, a next action and a
 * `job_id`. The role and the company live on the job, so the tracker joins the
 * two unparameterised lists it is given — `GET /api/applications` and
 * `GET /api/jobs` — rather than fetching a job per row, which would be an N+1
 * on every page load.
 */

type Status = components['schemas']['ApplicationStatus'];

/** Only the fields a row reads. Written out so a spec can build one by hand. */
export type Application = {
	id: string;
	job_id: string;
	status: Status;
	next_action: string;
	applied_at?: string | null;
};

export type Job = { id: string; title: string; company: string };

export type ApplicationStatusPill = {
	tone: 'neutral' | 'inflight' | 'live' | 'dormant' | 'danger';
	label: string;
};

/**
 * The ten statuses, in the tones the pill has.
 *
 * The tone answers "is this still alive?" before the label answers "what stage
 * is it at?" — which is the order someone scanning the column reads them in.
 * Amber is every stage that is still moving, so a tracker of twenty rows shows
 * at a glance how many are actually in play.
 */
const STATUSES: Record<Status, ApplicationStatusPill> = {
	draft: { tone: 'neutral', label: 'Draft' },
	applied: { tone: 'inflight', label: 'Applied' },
	screening: { tone: 'inflight', label: 'Screening' },
	interview: { tone: 'inflight', label: 'Interview' },
	final: { tone: 'inflight', label: 'Final round' },
	offer: { tone: 'live', label: 'Offer' },
	accepted: { tone: 'live', label: 'Accepted' },
	rejected: { tone: 'danger', label: 'Rejected' },
	withdrawn: { tone: 'dormant', label: 'Withdrawn' },
	no_reply: { tone: 'dormant', label: 'No reply' }
};

/**
 * A status we do not know is shown as it came rather than folded into "Draft":
 * a variant added on the backend must not make a live application look dead.
 */
export function applicationStatus(status: Status): ApplicationStatusPill {
	return STATUSES[status] ?? { tone: 'neutral', label: status };
}

/**
 * The day an application went, as the tracker frames write it: "14 Aug", and
 * "18 Dec 2025" once it is from another year. `null` for a draft, so each
 * surface says "not sent" its own way — a dash in the table, words on a card.
 *
 * Day-first and English, because that is how every frame writes a date and
 * there is no interface-language setting yet to follow instead. When there is,
 * this is the one place to change. Dates are read in the viewer's time zone.
 */
const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];

export function appliedOn(
	appliedAt: string | null | undefined,
	now: Date = new Date()
): string | null {
	if (!appliedAt) return null;

	const when = new Date(appliedAt);
	if (Number.isNaN(when.getTime())) return null;

	// Three letters, always. `Intl` in `en-GB` now writes "Sept", which no frame
	// does and which would make one month the odd width out in a column.
	const day = `${when.getDate()} ${MONTHS[when.getMonth()]}`;
	return when.getFullYear() === now.getFullYear() ? day : `${day} ${when.getFullYear()}`;
}

export type TrackerRow = {
	id: string;
	role: string;
	company: string;
	status: ApplicationStatusPill;
	nextAction: string;
	/** The day it was sent, or `null` for a draft. */
	applied: string | null;
};

/**
 * The two lists, joined by `job_id`, in the order the API sent them — newest
 * first, which is the order a tracker is read in.
 *
 * An application whose job is missing keeps its row. The jobs list is a second
 * request that can fail on its own, and dropping the row would hide an
 * application someone actually made.
 */
export function trackerRows(
	applications: Application[],
	jobs: Job[],
	now: Date = new Date()
): TrackerRow[] {
	const byId = new Map(jobs.map((job) => [job.id, job]));

	return applications.map((application) => {
		const job = byId.get(application.job_id);

		return {
			id: application.id,
			role: job?.title || 'Untitled role',
			company: job?.company ?? '',
			status: applicationStatus(application.status),
			nextAction: application.next_action,
			applied: appliedOn(application.applied_at, now)
		};
	});
}
