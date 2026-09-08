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

test('when it changed is the shared sentence, not a second one', () => {
	// The wording itself is covered in shared/lib/relative-time.spec.ts; what
	// matters here is that the posts table asks for it rather than rolling
	// its own and drifting from the tracker.
	expect(updatedLabel('2026-09-08T07:00:00Z', NOW)).toBe('5 hours ago');
	expect(updatedLabel('not-a-date', NOW)).toBe('—');
});
