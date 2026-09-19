import { expect, test } from 'vitest';
import { coverOf, pillFor, type Attachment } from './media';

const row = (over: Partial<Attachment> = {}): Attachment => ({
	media_id: 'm-1',
	attachment_target_id: 'post-1',
	role: 'cover',
	status: 'ready',
	alt_text: 'A hexagonal diagram',
	...over
});

test('finds the cover belonging to this post', () => {
	expect(coverOf([row()], 'post-1')?.media_id).toBe('m-1');
});

test('ignores another post’s cover', () => {
	// `GET /api/media/by-target/blog_post` lists every blog-post attachment the
	// caller owns, not this post's — the filter is the client's job.
	expect(coverOf([row({ attachment_target_id: 'post-2' })], 'post-1')).toBe(null);
});

test('ignores the post’s other images', () => {
	// The same post carries inline images too. §03: "Inline is a distinct role
	// from Cover — the same post has both, and they render differently."
	const rows = [row({ media_id: 'm-inline', role: 'inline' }), row({ media_id: 'm-cover' })];

	expect(coverOf(rows, 'post-1')?.media_id).toBe('m-cover');
});

test('a post with no cover has none, rather than an empty-looking one', () => {
	expect(coverOf([], 'post-1')).toBe(null);
});

test('a cover still processing is still the cover', () => {
	// It is what the post has. Hiding it until `ready` would make an upload
	// look lost and invite a second one.
	expect(coverOf([row({ status: 'processing' })], 'post-1')?.status).toBe('processing');
});

test('work in flight reads as in flight, and failure as failure', () => {
	// §03's MediaTile row, and the StatusPill rule: one tone per meaning.
	expect(pillFor('pending')).toEqual({ tone: 'inflight', label: 'Pending' });
	expect(pillFor('processing')).toEqual({ tone: 'inflight', label: 'Processing' });
	expect(pillFor('failed')).toEqual({ tone: 'danger', label: 'Failed' });
});

test('a ready image is shown rather than labelled', () => {
	// The picture is the status. A pill over it would be a caption saying "this
	// is a picture".
	expect(pillFor('ready')).toBe(null);
});
