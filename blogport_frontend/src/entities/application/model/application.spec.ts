import { expect, test } from 'vitest';
import { applicationStatus, appliedOn, cvUsed, nextAction, trackerRows } from './application';

/**
 * What one row of the application tracker can say about itself.
 *
 * `ApplicationResponse` carries a status, two dates and a `job_id`; the role
 * and the company live on the job. The tracker joins the two lists it is given
 * — `GET /api/applications` and `GET /api/jobs`, both unparameterised — rather
 * than fetching a job per row, which would be an N+1 on every page load.
 */

const NOW = new Date('2026-09-08T12:00:00Z');

// ── status ─────────────────────────────────────────────────────────────────

test('every status the API can send has a label a person would use', () => {
	const statuses = [
		'draft',
		'applied',
		'screening',
		'interview',
		'final',
		'offer',
		'accepted',
		'rejected',
		'withdrawn',
		'no_reply'
	] as const;

	for (const status of statuses) {
		const pill = applicationStatus(status);
		expect(pill.label).not.toBe('');
		expect(pill.label).not.toContain('_');
	}
});

test('the tones separate what is moving from what has stopped', () => {
	// The pill is the column someone scans; it has to answer "is this alive?"
	// before it answers "what stage is it at?".
	expect(applicationStatus('draft').tone).toBe('neutral');
	expect(applicationStatus('interview').tone).toBe('inflight');
	expect(applicationStatus('offer').tone).toBe('live');
	expect(applicationStatus('accepted').tone).toBe('live');
	expect(applicationStatus('rejected').tone).toBe('danger');
	expect(applicationStatus('withdrawn').tone).toBe('dormant');
	expect(applicationStatus('no_reply').tone).toBe('dormant');
});

test('a status we have never heard of is shown, not swallowed', () => {
	// A new variant on the backend must not make a row look like a draft.
	expect(applicationStatus('interviewing_with_ceo' as 'draft')).toEqual({
		tone: 'neutral',
		label: 'interviewing_with_ceo'
	});
});

// ── when it went out ───────────────────────────────────────────────────────

test('a draft has no applied date, and does not pretend to', () => {
	// Screen / Application tracker draws a dash in the Applied column for the
	// draft row. `null` here, so each surface can say it its own way.
	expect(appliedOn(null, NOW)).toBeNull();
	expect(appliedOn(undefined, NOW)).toBeNull();
});

test('a sent application says the day it went, in the reader’s own format', async () => {
	// Not a hand-rolled "14 Aug": the designer overruled that, and the Career
	// Studio paper warns about exactly this once Indonesian is added. The month
	// is whatever the locale writes — "Sept" in en-GB is correct.
	const day = new Intl.DateTimeFormat(undefined, { day: 'numeric', month: 'short' });

	expect(appliedOn('2026-09-06T12:00:00Z', NOW)).toBe(day.format(new Date('2026-09-06T12:00:00Z')));
});

test('a date from another year says which year', () => {
	const withYear = new Intl.DateTimeFormat(undefined, {
		day: 'numeric',
		month: 'short',
		year: 'numeric'
	});
	const when = new Date('2025-12-18T12:00:00Z');

	expect(appliedOn(when.toISOString(), NOW)).toBe(withYear.format(when));
	expect(appliedOn(when.toISOString(), NOW)).toContain('2025');
});

// ── the join ───────────────────────────────────────────────────────────────

const JOBS = [
	{ id: 'job-1', title: 'Senior Backend', company: 'Gojek' },
	{ id: 'job-2', title: 'Platform Engineer', company: 'Xendit' }
];

const APPLICATION = {
	id: 'app-1',
	job_id: 'job-1',
	status: 'interview' as const,
	next_action: 'Send the take-home',
	applied_at: '2026-09-06T12:00:00Z',
	created_at: '2026-09-01T12:00:00Z',
	updated_at: '2026-09-06T12:00:00Z'
};

test('a row carries its next action, and says whether it was derived', () => {
	const [row] = trackerRows([{ ...APPLICATION, status: 'rejected', next_action: '' }], JOBS, NOW);

	expect(row.nextAction).toEqual({ text: 'Add reflection', derived: true });
});

test('a row carries the role and the company from its job', () => {
	const [row] = trackerRows([APPLICATION], JOBS, NOW);

	expect(row.role).toBe('Senior Backend');
	expect(row.company).toBe('Gojek');
	expect(row.status.label).toBe('Interview');
	expect(row.applied).toBe(
		new Intl.DateTimeFormat(undefined, { day: 'numeric', month: 'short' }).format(
			new Date('2026-09-06T12:00:00Z')
		)
	);
	expect(row.nextAction).toEqual({ text: 'Send the take-home', derived: false });
});

test('an application whose job did not come back is still a row', () => {
	// Dropping it would hide an application someone made, and the job list is a
	// second request that can fail on its own.
	const [row] = trackerRows([{ ...APPLICATION, job_id: 'job-gone' }], JOBS, NOW);

	expect(row.id).toBe('app-1');
	expect(row.role).toBe('Untitled role');
	expect(row.company).toBe('');
});

test('nothing due is null, and each surface draws its own dash', () => {
	expect(trackerRows([{ ...APPLICATION, next_action: '' }], JOBS, NOW)[0].nextAction).toEqual({
		text: null,
		derived: false
	});
});

test('rows keep the order the API sent them in — newest first', () => {
	const second = { ...APPLICATION, id: 'app-2', job_id: 'job-2' };
	expect(trackerRows([APPLICATION, second], JOBS, NOW).map((row) => row.id)).toEqual([
		'app-1',
		'app-2'
	]);
});

// ── what the next-action cell holds ────────────────────────────────────────

test('a draft with no CV snapshot owes the tailoring step', () => {
	// §07 of the tracker rules: the derived action is chosen by status, never
	// by what the person typed.
	expect(
		nextAction({ ...APPLICATION, status: 'draft', cv_snapshot_id: null, next_action: '' })
	).toEqual({
		text: 'Tailor CV',
		derived: true
	});
});

test('a negative outcome with nothing written owes a reflection', () => {
	for (const status of ['rejected', 'withdrawn', 'no_reply'] as const) {
		expect(nextAction({ ...APPLICATION, status, next_action: '' }), status).toEqual({
			text: 'Add reflection',
			derived: true
		});
	}
});

test('the person’s own words win over a derived one', () => {
	// The derived action is what to do when the cell would otherwise be empty;
	// it never overwrites something someone wrote.
	expect(
		nextAction({ ...APPLICATION, status: 'rejected', next_action: 'Ask for feedback' })
	).toEqual({
		text: 'Ask for feedback',
		derived: false
	});
});

test('nothing owed and nothing written is a dash for the surface to draw', () => {
	expect(nextAction({ ...APPLICATION, status: 'interview', next_action: '' })).toEqual({
		text: null,
		derived: false
	});
});

test('a draft that has a snapshot is past the tailoring step', () => {
	expect(
		nextAction({ ...APPLICATION, status: 'draft', cv_snapshot_id: 'cv-1', next_action: '' })
	).toEqual({
		text: null,
		derived: false
	});
});

// ── CV used ────────────────────────────────────────────────────────────────

test('the CV column names the CV as it stood when it was sent', () => {
	// Screen / Application tracker 12:118: "Backend, sent 14 Aug". The role comes
	// from the frozen snapshot, so renaming the CV afterwards cannot rewrite what
	// this row says was sent.
	expect(cvUsed({ ...APPLICATION, cv_snapshot_id: 'snap-1', cv_role: 'Backend' }, '14 Aug')).toBe(
		'Backend, sent 14 Aug'
	);
});

test('a draft has not chosen one yet, and says exactly that', () => {
	expect(cvUsed({ ...APPLICATION, cv_snapshot_id: null, cv_role: null }, null)).toBe(
		'not chosen yet'
	);
});

test('a CV that had no role is still a CV that was sent', () => {
	// `null` rather than "" when the CV had no role — the one absent case the
	// backend called out. Naming no role beats inventing one.
	expect(cvUsed({ ...APPLICATION, cv_snapshot_id: 'snap-1', cv_role: null }, '14 Aug')).toBe(
		'sent 14 Aug'
	);
});

test('a tracker row carries the CV cell already written', () => {
	const rows = trackerRows(
		[{ ...APPLICATION, id: 'a1', job_id: 'j1', cv_snapshot_id: 'snap-1', cv_role: 'Backend' }],
		[{ id: 'j1', title: 'Senior Backend', company: 'Gojek' }],
		NOW
	);

	expect(rows[0].cvUsed).toContain('Backend, sent ');
});
