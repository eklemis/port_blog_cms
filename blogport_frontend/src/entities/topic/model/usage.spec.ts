import { expect, test } from 'vitest';
import { retireQuestion, usageLine, usageParts } from './usage';

/**
 * What a retire confirmation asks.
 *
 * §02 writes the sentence: "Retire «Rust»? It's on 6 posts and 2 projects."
 * and the rule under it — "Never drop a topic off eight pages silently."
 *
 * The numbers come from `GET /api/topics/{id}/usage`, which exists precisely
 * for this: "Getting that number previously meant fetching every post and
 * project and their topics, so the console either warned generically or
 * invented a figure."
 */

test('names the topic and both counts, as the blueprint writes it', () => {
	expect(retireQuestion('Rust', { posts: 6, projects: 2 })).toBe(
		'Retire «Rust»? It’s on 6 posts and 2 projects.'
	);
});

test('counts of one are not counts of many', () => {
	expect(retireQuestion('Rust', { posts: 1, projects: 1 })).toBe(
		'Retire «Rust»? It’s on 1 post and 1 project.'
	);
});

test('a kind with nothing in it is left out rather than written as a zero', () => {
	// "on 6 posts and 0 projects" makes a reader check the zero.
	expect(retireQuestion('Rust', { posts: 6, projects: 0 })).toBe('Retire «Rust»? It’s on 6 posts.');
	expect(retireQuestion('Rust', { posts: 0, projects: 2 })).toBe(
		'Retire «Rust»? It’s on 2 projects.'
	);
});

test('an unused topic says so, rather than asking a question with no stakes', () => {
	expect(retireQuestion('Rust', { posts: 0, projects: 0 })).toBe(
		'Retire «Rust»? Nothing is using it.'
	);
});

test('counts that never arrived are not invented', () => {
	// The endpoint can fail. Warning generically is what it replaced, so saying
	// nothing about numbers beats saying a wrong one.
	expect(retireQuestion('Rust', null)).toBe('Retire «Rust»?');
});

test('the title is quoted as the blueprint quotes it, whatever is in it', () => {
	expect(retireQuestion('C++', { posts: 1, projects: 0 })).toContain('«C++»');
});

/**
 * The column and the sentence share their arithmetic.
 *
 * The backend's warning when it shipped the counts: the row and the retire
 * confirmation must be "the same number from the same rule — not two counts
 * that agree until someone archives a post". Sharing the parts is how that is
 * guaranteed here rather than hoped for.
 */

test('the column lists the same parts the sentence does', () => {
	expect(usageParts({ posts: 6, projects: 2 })).toEqual(['6 posts', '2 projects']);
});

test('a kind with nothing in it is left out of both', () => {
	expect(usageParts({ posts: 6, projects: 0 })).toEqual(['6 posts']);
	expect(usageParts({ posts: 0, projects: 0 })).toEqual([]);
});

test('the column reads as the frame writes it', () => {
	// 70:382: "6 posts · 2 projects", "3 posts", "1 post".
	expect(usageLine({ posts: 6, projects: 2 })).toBe('6 posts · 2 projects');
	expect(usageLine({ posts: 1, projects: 0 })).toBe('1 post');
});

test('a topic on nothing says so rather than showing a zero', () => {
	// "0 posts · 0 projects" is a row that makes a reader check two numbers to
	// learn one thing.
	expect(usageLine({ posts: 0, projects: 0 })).toBe('Not used yet');
});
