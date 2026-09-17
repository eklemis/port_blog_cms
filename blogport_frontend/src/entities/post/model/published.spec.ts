import { expect, test } from 'vitest';
import { publishedLabel, readingTime } from './published';

/** Screen / Public post 72:17: "14 August 2026 · 7 min read". */

test('the date is the reader’s, not a hand-rolled English one', () => {
	// Overruled once already on the tracker, for the reason that applies here
	// too: a hand-coded format is the bug that surfaces the day a second locale
	// is added. Intl is asked for the parts; it decides the order and the words.
	const on = '2026-08-14T09:30:00Z';

	expect(publishedLabel(on, 'en-GB')).toBe('14 August 2026');
	expect(publishedLabel(on, 'en-US')).toBe('August 14, 2026');
});

test('an unpublished post has no date to print', () => {
	expect(publishedLabel(null)).toBe(null);
	expect(publishedLabel('not a date')).toBe(null);
});

test('reading time counts words, and rounds up to a minute', () => {
	// 200 words a minute. Anything a person can read at all is "1 min read" —
	// "0 min read" describes nothing.
	expect(readingTime(Array(600).fill('word').join(' '))).toBe(3);
	expect(readingTime('Three words here')).toBe(1);
	expect(readingTime('')).toBe(1);
});

test('reading time counts the prose, not the punctuation around it', () => {
	// Markdown marks are not words a person reads. A heading of "## Layout" is
	// one word, and a fenced block's backticks are none.
	expect(readingTime('## Layout\n\n`cargo check`')).toBe(1);
	expect(readingTime(`![alt](media:8f1b2c3d)\n\n${Array(400).fill('word').join(' ')}`)).toBe(2);
});
