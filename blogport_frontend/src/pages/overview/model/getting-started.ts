import { CONSOLE_ROUTES } from '$lib/shared/config/routes';

/**
 * First run: three things, then get out of the way.
 *
 * Console Blueprint §05 — "a dismissible checklist on /studio: create a topic,
 * write a first post, add a résumé. It disappears for good once any two are
 * done; it is not a permanent fixture." Everything here is derived from counts
 * the screen already has, so there is no state to store beyond the dismissal.
 */

/** `done: null` is "we could not tell", which renders unticked but does not count. */
export type Task = { label: string; href: string; done: boolean | null };

/** `null` where the count could not be fetched — which is not zero. */
export type Counts = {
	topics: number | null;
	posts: number | null;
	resumes: number | null;
};

const known = (count: number | null) => (count === null ? null : count > 0);

export function gettingStarted(counts: Counts): Task[] {
	return [
		{ label: 'Create a topic', href: CONSOLE_ROUTES.topics, done: known(counts.topics) },
		{
			label: 'Write your first post',
			href: `${CONSOLE_ROUTES.posts}/new`,
			done: known(counts.posts)
		},
		{ label: 'Add a résumé', href: CONSOLE_ROUTES.resumes, done: known(counts.resumes) }
	];
}

/**
 * Whether the checklist belongs on the screen at all.
 *
 * Two done and it is finished with. Dismissed and it is finished with. And if
 * nothing could be counted it stays away: telling someone with forty posts to
 * write their first one because a request failed is the worse way to be wrong.
 */
export function showGettingStarted(tasks: Task[], dismissed: boolean): boolean {
	if (dismissed) return false;
	if (tasks.every((task) => task.done === null)) return false;

	const done = tasks.filter((task) => task.done === true).length;
	return done < 2;
}
