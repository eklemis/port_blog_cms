/**
 * "Good morning, Jane".
 *
 * The frame shows a greeting that names the hour, so it is derived from the
 * viewer's own clock rather than the server's — a console opened at breakfast
 * in Jakarta should not say good evening because the server is in Frankfurt.
 * Nothing in the blueprint specifies the wording; see the PR.
 */
export function greeting(fullName: string, at: Date = new Date()): string {
	const hour = at.getHours();
	const partOfDay = hour < 12 ? 'morning' : hour < 18 ? 'afternoon' : 'evening';
	const first = fullName.trim().split(/\s+/)[0];

	// A name is not a format: someone with one word for a name gets it whole.
	return first ? `Good ${partOfDay}, ${first}` : `Good ${partOfDay}`;
}
