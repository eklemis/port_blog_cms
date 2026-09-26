import type { components } from '$lib/shared/api/v1';

/**
 * What a topic is attached to, and what a retire confirmation asks.
 *
 * §02 writes the sentence and the rule together: "Retire «Rust»? It's on 6
 * posts and 2 projects." — "Never drop a topic off eight pages silently."
 *
 * The numbers are real, from `GET /api/topics/{id}/usage`. That endpoint exists
 * for this sentence: "Getting that number previously meant fetching every post
 * and project and their topics, so the console either warned generically or
 * invented a figure."
 */

export type TopicUsage = components['schemas']['TopicUsage'];

/** "6 posts", "1 post" — the count and its noun, agreeing. */
function count(n: number, noun: 'post' | 'project'): string {
	return `${n} ${noun}${n === 1 ? '' : 's'}`;
}

/**
 * The counted kinds, each named once.
 *
 * Shared by the column and the confirmation on purpose. When the counts shipped
 * on the listing, the backend's warning was that the row and the retire
 * sentence must be "the same number from the same rule — not two counts that
 * agree until someone archives a post". One formatter is how that is
 * guaranteed rather than hoped for.
 *
 * A kind with nothing in it is left out rather than written as a zero: "6 posts
 * and 0 projects" makes a reader stop and check the zero.
 */
export function usageParts(usage: TopicUsage): string[] {
	return [
		usage.posts > 0 ? count(usage.posts, 'post') : null,
		usage.projects > 0 ? count(usage.projects, 'project') : null
	].filter((part): part is string => part !== null);
}

/** The Used on cell — 70:382 writes it "6 posts · 2 projects". */
export function usageLine(usage: TopicUsage): string {
	const parts = usageParts(usage);

	return parts.length ? parts.join(' · ') : 'Not used yet';
}

export function retireQuestion(title: string, usage: TopicUsage | null): string {
	const asked = `Retire «${title}»?`;

	// The endpoint can fail, and warning generically is exactly what it
	// replaced. Saying nothing about numbers beats saying a wrong one.
	if (!usage) return asked;

	const parts = usageParts(usage);

	if (!parts.length) return `${asked} Nothing is using it.`;

	return `${asked} It’s on ${parts.join(' and ')}.`;
}
