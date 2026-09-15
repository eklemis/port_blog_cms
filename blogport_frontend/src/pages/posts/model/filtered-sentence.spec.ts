import { expect, test } from 'vitest';
import { filteredSentence } from './filtered-sentence';

/**
 * The sentence under "No posts match those filters".
 *
 * Screen / Posts — filtered empty: "You have 24 posts — none of them are drafts
 * tagged “Rust”." It exists so nobody thinks their work is gone: it says what
 * there is, and then says which filter is the reason there is nothing here.
 */

test('the frame’s own case', () => {
	expect(filteredSentence({ everything: 24, published: 'false', topic: 'Rust' })).toBe(
		'You have 24 posts — none of them are drafts tagged “Rust”.'
	);
});

test('a status alone', () => {
	expect(filteredSentence({ everything: 24, published: 'true' })).toBe(
		'You have 24 posts — none of them are published.'
	);
});

test('a topic alone', () => {
	expect(filteredSentence({ everything: 3, topic: 'Rust' })).toBe(
		'You have 3 posts — none of them are tagged “Rust”.'
	);
});

test('a search alone', () => {
	expect(filteredSentence({ everything: 24, search: 'kafka' })).toBe(
		'You have 24 posts — none of them match “kafka”.'
	);
});

test('a search on top of the rest', () => {
	expect(
		filteredSentence({ everything: 24, published: 'false', topic: 'Rust', search: 'kafka' })
	).toBe('You have 24 posts — none of them are drafts tagged “Rust” matching “kafka”.');
});

test('one post is one post', () => {
	expect(filteredSentence({ everything: 1, published: 'true' })).toBe(
		'You have 1 post — it is not published.'
	);
});

test('a count we could not get is left out rather than guessed', () => {
	// Saying "0 posts" here would be the exact misreading this sentence is for.
	expect(filteredSentence({ everything: null, published: 'false' })).toBe(
		'None of your posts are drafts.'
	);
});
