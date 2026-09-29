/**
 * What a résumé says about itself in a list — Screen / Résumés 241:5528.
 *
 * "6 roles · 3 projects": the two collections a reader scans for. The counts
 * come off the rows the listing already returns, so the cell is a render rather
 * than a request.
 *
 * There is no Updated column beside it, and that is deliberate rather than
 * missed: `CvResponse` carries no `updated_at` while `CVSort` offers
 * `updated_newest`, so the list can be *sorted* by a date it cannot *show*.
 * Filed with the backend.
 */

/** "6 roles", "1 role" — the count and its noun, agreeing. */
function count(n: number, noun: 'role' | 'project'): string {
	return `${n} ${noun}${n === 1 ? '' : 's'}`;
}

export function sectionsLine(cv: {
	experiences: unknown[];
	highlighted_projects: unknown[];
}): string {
	const parts = [
		cv.experiences.length > 0 ? count(cv.experiences.length, 'role') : null,
		cv.highlighted_projects.length > 0 ? count(cv.highlighted_projects.length, 'project') : null
	].filter((part): part is string => part !== null);

	// A section with nothing in it is left out rather than written as a zero —
	// the same rule the topics column follows.
	if (!parts.length) return 'Nothing on it yet';

	return parts.join(' · ');
}
