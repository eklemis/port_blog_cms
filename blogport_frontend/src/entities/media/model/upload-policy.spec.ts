import { expect, test } from 'vitest';
import {
	ACCEPTED_TYPES,
	MAX_BYTES,
	MAX_EDGE,
	checkDeclared,
	checkDimensions
} from './upload-policy';

/**
 * §03 Media upload: "≤ 5 MB, ≤ 6000px/side, JPEG/PNG/WebP — all three checked
 * in the browser first."
 *
 * The reason this is not belt-and-braces: the bytes never reach the API. They
 * go straight to a signed GCS URL, and `InitUploadRequest.mime_type` is
 * documented as "**as declared by the client**. Never checked against the
 * bytes." So the browser is not duplicating a server check here — for the
 * dimensions it is the only check there is.
 */

test('a file inside every limit is accepted', () => {
	expect(checkDeclared({ name: 'hero.png', size: 400_000, type: 'image/png' })).toBe(null);
});

test('an oversized file is refused, and told what the limit is', () => {
	const rejection = checkDeclared({ name: 'hero.png', size: 8_600_000, type: 'image/png' });

	expect(rejection?.code).toBe('FILE_TOO_LARGE');
	// §07: never blames the person. It states the rule and the measurement.
	expect(rejection?.message).toBe('Images must be 5 MB or smaller. That one is 8.2 MB.');
});

test('a file of the wrong kind is refused by kind, not by extension', () => {
	// The extension is the writer's to get wrong; the type is what the upload
	// policy is written in.
	const rejection = checkDeclared({ name: 'animation.gif', size: 90_000, type: 'image/gif' });

	expect(rejection?.code).toBe('INVALID_MIME_TYPE');
	expect(rejection?.message).toBe('Images must be a JPEG, PNG or WebP.');
});

test('the three accepted types are exactly the ones the server policy names', () => {
	expect([...ACCEPTED_TYPES]).toEqual(['image/jpeg', 'image/png', 'image/webp']);
});

test('size is checked before kind, so one file yields one reason', () => {
	// A 9 MB GIF is both. Reporting both would make the drop zone a list of
	// faults; the first one is enough to act on.
	const rejection = checkDeclared({ name: 'big.gif', size: 9_000_000, type: 'image/gif' });

	expect(rejection?.code).toBe('FILE_TOO_LARGE');
});

test('an image inside the edge limit passes', () => {
	expect(checkDimensions(1600, 900)).toBe(null);
	expect(checkDimensions(MAX_EDGE, MAX_EDGE)).toBe(null);
});

test('an image over the edge limit is refused on either side', () => {
	expect(checkDimensions(7000, 900)?.code).toBe('INVALID_DIMENSIONS');
	expect(checkDimensions(900, 7000)?.code).toBe('INVALID_DIMENSIONS');
	expect(checkDimensions(7000, 900)?.message).toBe(
		'Images must be 6000px or smaller on each side. That one is 7000px.'
	);
});

test('the limits are the ones the API documents', () => {
	// 5 MB and 6000px come from the server-side upload policy quoted on
	// `POST /api/media/upload-url`. Named here so a drift is one edit.
	expect(MAX_BYTES).toBe(5 * 1024 * 1024);
	expect(MAX_EDGE).toBe(6000);
});
