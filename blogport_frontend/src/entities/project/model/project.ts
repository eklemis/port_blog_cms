import type { components } from '$lib/shared/api/v1';

/**
 * A project: a piece of work with an address, a stack and somewhere to see it.
 *
 * Unlike a post it has no draft state — creating one publishes it. The console
 * says so out loud on the list rather than letting someone discover it, which
 * is the notice Screen / Projects list 70:2 carries above the table.
 */

export type ProjectSort = components['schemas']['ProjectSort'];

/** What a listing row needs. Narrower than the wire type on purpose. */
export type ProjectCard = {
	id: string;
	title: string;
	slug: string;
	description?: string | null;
	tech_stack: string[];
	topics: { id: string; title: string }[];
	repo_url?: string | null;
	live_demo_url?: string | null;
	updated_at: string;
};

export type ProjectLink = { label: 'repo' | 'demo'; href: string };

/**
 * The Links cell.
 *
 * Repo first on every row that has both, because someone scanning a portfolio
 * of engineering work is usually after the code. A project may have neither,
 * and then the cell is empty rather than a lone separator.
 */
export function linksOf(project: ProjectCard): ProjectLink[] {
	const links: ProjectLink[] = [];

	// Trimmed, not merely checked for null: an empty string survives a round
	// trip through the API as a value, and it is not somewhere to go.
	if (project.repo_url?.trim()) links.push({ label: 'repo', href: project.repo_url.trim() });
	if (project.live_demo_url?.trim())
		links.push({ label: 'demo', href: project.live_demo_url.trim() });

	return links;
}
