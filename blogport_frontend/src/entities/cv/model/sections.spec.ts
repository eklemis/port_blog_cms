import { expect, test } from 'vitest';
import { sectionsLine } from './sections';

/**
 * The Sections cell — Screen / Résumés 241:5528 writes it "6 roles · 3
 * projects".
 *
 * It is what a CV can say about itself in a list. There is no Updated column
 * on purpose: `CvResponse` carries no `updated_at`, while `CVSort` offers
 * `updated_newest` — so the list can be sorted by a date it cannot show.
 */

const cv = (experiences: number, projects: number) => ({
	experiences: Array.from({ length: experiences }, () => ({})),
	highlighted_projects: Array.from({ length: projects }, () => ({}))
});

test('counts the two things a reader scans a résumé for', () => {
	expect(sectionsLine(cv(6, 3))).toBe('6 roles · 3 projects');
});

test('counts of one are not counts of many', () => {
	expect(sectionsLine(cv(1, 1))).toBe('1 role · 1 project');
});

test('a section with nothing in it is left out rather than written as a zero', () => {
	// The same rule as the topics column: a zero makes a reader check a number
	// in order to learn there is nothing to check.
	expect(sectionsLine(cv(4, 0))).toBe('4 roles');
	expect(sectionsLine(cv(0, 2))).toBe('2 projects');
});

test('a résumé with neither says it is empty, not that it has none of two things', () => {
	expect(sectionsLine(cv(0, 0))).toBe('Nothing on it yet');
});
