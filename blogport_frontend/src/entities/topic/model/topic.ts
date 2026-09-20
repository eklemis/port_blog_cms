/**
 * A topic: a label an author puts on their own work.
 *
 * Lives here rather than in either editor because posts and projects both carry
 * them, and slices in the same layer may not import each other. The attach and
 * detach endpoints differ per target — `/api/blog/{id}/topics` against
 * `/api/projects/{id}/topics` — so those stay with the feature that owns them,
 * and only the shape and the picker are shared.
 */
export type Topic = { id: string; title: string };
