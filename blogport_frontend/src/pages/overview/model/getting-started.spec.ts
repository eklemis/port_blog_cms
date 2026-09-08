import { expect, test } from 'vitest';
import { gettingStarted, showGettingStarted } from './getting-started';

/**
 * First run: three things, then get out of the way.
 *
 * The blueprint is explicit that this is not a permanent fixture — it goes for
 * good once two of the three are done, and it can be dismissed before that.
 */

const NONE = { topics: 0, posts: 0, resumes: 0 };

test('the three things are the three the blueprint names, in its order', () => {
	expect(gettingStarted(NONE).map((task) => task.label)).toEqual([
		'Create a topic',
		'Write your first post',
		'Add a résumé'
	]);
});

test('each one links to the screen that does it', () => {
	const [topic, post, resume] = gettingStarted(NONE);

	expect(topic.href).toBe('/studio/topics');
	expect(post.href).toBe('/studio/posts/new');
	expect(resume.href).toBe('/studio/resumes');
});

test('a thing that exists is ticked', () => {
	const tasks = gettingStarted({ topics: 3, posts: 0, resumes: 0 });

	expect(tasks[0].done).toBe(true);
	expect(tasks[1].done).toBe(false);
});

// ── when it goes away ──────────────────────────────────────────────────────

test('it is there while at most one thing is done', () => {
	expect(showGettingStarted(gettingStarted(NONE), false)).toBe(true);
	expect(showGettingStarted(gettingStarted({ ...NONE, posts: 1 }), false)).toBe(true);
});

test('two done and it is gone for good', () => {
	expect(showGettingStarted(gettingStarted({ topics: 1, posts: 1, resumes: 0 }), false)).toBe(
		false
	);
});

test('dismissing it is enough on its own', () => {
	expect(showGettingStarted(gettingStarted(NONE), true)).toBe(false);
});

test('a count we could not fetch is not a thing left undone', () => {
	// Showing "write your first post" to someone with forty posts because one
	// request failed is the worse way to be wrong.
	const tasks = gettingStarted({ topics: null, posts: null, resumes: null });

	expect(showGettingStarted(tasks, false)).toBe(false);
});

test('one count surviving is still enough to say something', () => {
	const tasks = gettingStarted({ topics: null, posts: 0, resumes: 0 });

	// Unknown is not done — it renders unticked — but it is not a lie either,
	// which is why it stays distinguishable from a real zero.
	expect(showGettingStarted(tasks, false)).toBe(true);
	expect(tasks[0].done).not.toBe(true);
});
