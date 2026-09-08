import { expect, test } from 'vitest';
import { postStatus, updatedLabel } from './post';

/**
 * What a post row can say about itself.
 *
 * Only what the list endpoint actually returns: `BlogPostCardResponse` carries
 * `published_at` and no status field at all, so live, scheduled and draft are
 * derived from that one timestamp. Archived is not derivable — see the PR.
 */

const NOW = new Date('2026-09-08T12:00:00Z');

// ── status ─────────────────────────────────────────────────────────────────

test('no publish date is a draft', () => {
	expect(postStatus(null, NOW)).toEqual({ tone: 'neutral', label: 'Draft' });
	expect(postStatus(undefined, NOW)).toEqual({ tone: 'neutral', label: 'Draft' });
});

test('a publish date in the past is live', () => {
	expect(postStatus('2026-09-01T09:00:00Z', NOW)).toEqual({ tone: 'live', label: 'Live' });
});

test('a publish date in the future is scheduled, not live', () => {
	// The difference someone plans around: it says the post is not out yet.
	expect(postStatus('2026-09-09T09:00:00Z', NOW)).toEqual({
		tone: 'inflight',
		label: 'Scheduled'
	});
});

test('the boundary is now, and now counts as published', () => {
	expect(postStatus(NOW.toISOString(), NOW).label).toBe('Live');
});

test('an unparseable date is not silently called live', () => {
	// Better to say draft than to claim something is public when we cannot tell.
	expect(postStatus('not-a-date', NOW).label).toBe('Draft');
});

// ── when it changed ────────────────────────────────────────────────────────

test('recent changes read in the units people use', () => {
	expect(updatedLabel('2026-09-08T07:00:00Z', NOW)).toBe('5 hours ago');
	expect(updatedLabel('2026-09-06T12:00:00Z', NOW)).toBe('2 days ago');
	expect(updatedLabel('2026-09-07T12:00:00Z', NOW)).toBe('yesterday');
});

test('weeks are weeks, not twenty-one days', () => {
	expect(updatedLabel('2026-08-18T12:00:00Z', NOW)).toBe('3 weeks ago');
});

test('anything old enough gets a date instead of a countdown', () => {
	// "34 weeks ago" is arithmetic, not information.
	expect(updatedLabel('2026-04-02T12:00:00Z', NOW)).toBe('Apr 2026');
});

test('a missing or broken timestamp says nothing rather than lying', () => {
	expect(updatedLabel('not-a-date', NOW)).toBe('—');
});
