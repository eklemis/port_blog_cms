import { expect, test } from 'vitest';
import {
	TITLE_COUNTER_FROM,
	TITLE_MAX,
	postStatus,
	publicPostPath,
	titleError,
	updatedLabel
} from './post';

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

// ── the title ──────────────────────────────────────────────────────────────

test('a post needs a title, because the server will not take one without', () => {
	// `INVALID_TITLE` with "Title cannot be empty" — not the `EMPTY_TITLE` the
	// blueprint's branch list names, which blog never sends. See the PR.
	expect(titleError('')).toBeDefined();
	expect(titleError('   ')).toBeDefined();
});

test('the cap is 200, counted the way the server counts it', () => {
	// Rust validates on `chars().count()`, so an emoji is one and not two.
	expect(titleError('a'.repeat(TITLE_MAX))).toBeUndefined();
	expect(titleError('a'.repeat(TITLE_MAX + 1))).toBeDefined();
	expect(titleError('🌱'.repeat(TITLE_MAX))).toBeUndefined();
});

test('the cap is measured on the trimmed title, as the server measures it', () => {
	expect(titleError(`  ${'a'.repeat(TITLE_MAX)}  `)).toBeUndefined();
});

test('the counter appears before the cap does, not at it', () => {
	// J4: show a live counter from 160 characters onward rather than rejecting
	// at submit.
	expect(TITLE_COUNTER_FROM).toBe(160);
	expect(TITLE_COUNTER_FROM).toBeLessThan(TITLE_MAX);
});

// ── where it goes live ─────────────────────────────────────────────────────

test('the public address is the one the surface map gives', () => {
	// `/[username]/blog/[slug]`. J4's prose shortens it to `/{username}/{slug}`,
	// which is not a route in the map — see the PR.
	expect(publicPostPath('janedoe', 'building-a-cms')).toBe('/janedoe/blog/building-a-cms');
});

test('both halves are escaped, because both come from a person', () => {
	expect(publicPostPath('jane doe', 'a slug')).toBe('/jane%20doe/blog/a%20slug');
});
