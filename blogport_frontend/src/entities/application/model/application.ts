import { relativeDate } from '$lib/shared/lib/relative-time';
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

/** A draft has never been sent, and the column says so rather than going blank. */
export function appliedLabel(appliedAt: string | null | undefined, now: Date = new Date()): string {
	return appliedAt ? relativeDate(appliedAt, now) : 'Not sent';
}

export type TrackerRow = {
	id: string;
	role: string;
	company: string;
	status: ApplicationStatusPill;
	nextAction: string;
	applied: string;
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
			applied: appliedLabel(application.applied_at, now)
		};
	});
}
