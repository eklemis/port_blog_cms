import { expect, test } from 'vitest';
import { applicationStatus, appliedLabel, trackerRows } from './application';

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
	expect(appliedLabel(null, NOW)).toBe('Not sent');
	expect(appliedLabel(undefined, NOW)).toBe('Not sent');
});

test('a sent application says when, in the same words the posts list uses', () => {
	expect(appliedLabel('2026-09-06T12:00:00Z', NOW)).toBe('2 days ago');
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

test('a row carries the role and the company from its job', () => {
	const [row] = trackerRows([APPLICATION], JOBS, NOW);

	expect(row.role).toBe('Senior Backend');
	expect(row.company).toBe('Gojek');
	expect(row.status.label).toBe('Interview');
	expect(row.applied).toBe('2 days ago');
	expect(row.nextAction).toBe('Send the take-home');
});

test('an application whose job did not come back is still a row', () => {
	// Dropping it would hide an application someone made, and the job list is a
	// second request that can fail on its own.
	const [row] = trackerRows([{ ...APPLICATION, job_id: 'job-gone' }], JOBS, NOW);

	expect(row.id).toBe('app-1');
	expect(row.role).toBe('Untitled role');
	expect(row.company).toBe('');
});

test('nothing due is empty, not a dash someone has to read', () => {
	expect(trackerRows([{ ...APPLICATION, next_action: '' }], JOBS, NOW)[0].nextAction).toBe('');
});

test('rows keep the order the API sent them in — newest first', () => {
	const second = { ...APPLICATION, id: 'app-2', job_id: 'job-2' };
	expect(trackerRows([APPLICATION, second], JOBS, NOW).map((row) => row.id)).toEqual([
		'app-1',
		'app-2'
	]);
});
