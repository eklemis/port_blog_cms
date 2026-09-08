import { expect, test } from 'vitest';
import { relativeDate } from './relative-time';

/**
 * When something last happened, said the way people say it.
 *
 * Shared rather than owned by one entity: the posts list calls it "Updated" and
 * the application tracker calls it "Applied", and two entities may not import
 * each other, so the one behaviour lives below both.
 */

const NOW = new Date('2026-09-08T12:00:00Z');

test('recent changes read in the units people use', () => {
	expect(relativeDate('2026-09-08T07:00:00Z', NOW)).toBe('5 hours ago');
	expect(relativeDate('2026-09-06T12:00:00Z', NOW)).toBe('2 days ago');
	expect(relativeDate('2026-09-07T12:00:00Z', NOW)).toBe('yesterday');
});

test('weeks are weeks, not twenty-one days', () => {
	expect(relativeDate('2026-08-18T12:00:00Z', NOW)).toBe('3 weeks ago');
});

test('anything old enough gets a date instead of a countdown', () => {
	// "34 weeks ago" is arithmetic, not information.
	expect(relativeDate('2026-04-02T12:00:00Z', NOW)).toBe('Apr 2026');
});

test('a missing or broken timestamp says nothing rather than lying', () => {
	expect(relativeDate('not-a-date', NOW)).toBe('—');
	expect(relativeDate(null, NOW)).toBe('—');
	expect(relativeDate(undefined, NOW)).toBe('—');
});
