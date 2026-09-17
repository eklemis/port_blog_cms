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
	/** `null` while the application is a draft — the tailoring has not run. */
	cv_snapshot_id?: string | null;
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
 * The day an application went, in the reader's own format: `Intl` with day and
 * short month, and the year as well once it is from another year. `null` for a
 * draft, so each surface says "not sent" its own way.
 *
 * The locale's month, not a hand-written one. "Sept" is what `en-GB` writes and
 * it is correct; a hard-coded table of English months is the bug the Career
 * Studio paper warns about for the day Indonesian is added.
 */
export function appliedOn(
	appliedAt: string | null | undefined,
	now: Date = new Date()
): string | null {
	if (!appliedAt) return null;

	const when = new Date(appliedAt);
	if (Number.isNaN(when.getTime())) return null;

	const sameYear = when.getFullYear() === now.getFullYear();

	return new Intl.DateTimeFormat(undefined, {
		day: 'numeric',
		month: 'short',
		...(sameYear ? {} : { year: 'numeric' })
	}).format(when);
}

export type NextAction = {
	/** What the cell says. `null` when nothing is owed and nothing was written. */
	text: string | null;
	/** Derived from status rather than written by the person. */
	derived: boolean;
};

/**
 * What the next-action cell holds — §07 of the tracker rules.
 *
 * Two different things in one cell. Where the application owes a step, the cell
 * shows an action derived from the status and never from the text: a draft with
 * no CV snapshot owes the tailoring, and a rejected, withdrawn or unanswered
 * application owes a reflection. Otherwise it shows what the person wrote.
 *
 * Their words win. The derived action fills a cell that would be empty; it
 * never speaks over something someone typed.
 *
 * The rule the list cannot check: "with no reflection". The listing carries no
 * reflection, so an application that already has one still reads "Add
 * reflection" until its author writes something else. Reported.
 */
const OWES_REFLECTION = new Set<Status>(['rejected', 'withdrawn', 'no_reply']);

export function nextAction(application: Application): NextAction {
	const written = application.next_action?.trim();
	if (written) return { text: written, derived: false };

	if (application.status === 'draft' && !application.cv_snapshot_id) {
		return { text: 'Tailor CV', derived: true };
	}
	if (OWES_REFLECTION.has(application.status)) {
		return { text: 'Add reflection', derived: true };
	}

	return { text: null, derived: false };
}

export type TrackerRow = {
	id: string;
	role: string;
	company: string;
	status: ApplicationStatusPill;
	nextAction: NextAction;
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
			nextAction: nextAction(application),
			applied: appliedOn(application.applied_at, now)
		};
	});
}
