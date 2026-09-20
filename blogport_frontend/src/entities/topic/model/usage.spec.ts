import { expect, test } from 'vitest';
import { retireQuestion } from './usage';

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
