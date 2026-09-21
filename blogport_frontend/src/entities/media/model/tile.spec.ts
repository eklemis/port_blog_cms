import { expect, test } from 'vitest';
import { tileStatus } from './tile';

/**
 * What a media tile says about itself — Screen / Media library 69:242.
 *
 * The frame writes four lines, and the one that matters most is the prompt: an
 * image with no alt text says so, in accent ink, as something to fix rather
 * than a fact to read.
 */

const item = (over: Record<string, unknown> = {}) => ({
	media_id: 'm-1',
	original_filename: 'hexagonal-layout.png',
	role: 'cover' as const,
	status: 'ready' as const,
	alt_text: 'A hexagonal diagram',
	...over
});

test('a described image says so, and what it is for', () => {
	expect(tileStatus(item())).toEqual({ text: 'Alt text set · cover', prompt: false });
});

test('an image with no description asks for one, and it is the thing to press', () => {
	// The frame draws this in accent ink where the others are muted: it is a
	// job, not a fact.
	expect(tileStatus(item({ alt_text: '' }))).toEqual({
		text: 'No alt text — add one',
		prompt: true
	});
});

test('whitespace is not a description', () => {
	expect(tileStatus(item({ alt_text: '   ' })).prompt).toBe(true);
});

test('work still in flight says so rather than claiming to be ready', () => {
	expect(tileStatus(item({ status: 'processing' })).text).toBe('Processing');
	expect(tileStatus(item({ status: 'pending' })).text).toBe('Processing');
});

test('a failure says what failed, in the frame’s words', () => {
	expect(tileStatus(item({ status: 'failed' }))).toEqual({
		text: 'Processing failed',
		prompt: false
	});
});

test('a failed image is not also nagged about its alt text', () => {
	// There is nothing to describe yet, and two problems on one tile reads as
	// two jobs when there is one.
	expect(tileStatus(item({ status: 'failed', alt_text: '' })).prompt).toBe(false);
});
