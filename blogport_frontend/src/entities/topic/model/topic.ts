/**
 * A topic: a label an author puts on their own work.
 *
 * Lives here rather than in either editor because posts and projects both carry
 * them, and slices in the same layer may not import each other. The attach and
 * detach endpoints differ per target — `/api/blog/{id}/topics` against
 * `/api/projects/{id}/topics` — so those stay with the feature that owns them,
 * and only the shape and the picker are shared.
 */
export type Topic = {
	id: string;
	title: string;
	/**
	 * What the word means here. §02 asks for it beside the title wherever a
	 * topic is created: "a taxonomy of bare words stops being useful at about
	 * fifteen entries." Empty rather than absent when nothing was written.
	 */
	description?: string | null;
	/**
	 * How many live posts and projects carry this topic, counted in the same
	 * statement that lists them. Soft-deleted ones are excluded, which is the
	 * rule `…/usage` applies — so the column and the retire confirmation are one
	 * number from one definition.
	 */
	post_count?: number;
	project_count?: number;
};
