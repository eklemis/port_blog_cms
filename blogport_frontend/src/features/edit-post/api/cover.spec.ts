import { expect, test, vi } from 'vitest';
import { beginUpload, correctAltText, readState, removeCover, uploadBytes } from './cover';

/**
 * The cover upload, §03's "Inserting an image" flow with `role: cover`.
 *
 * Four calls, deliberately not one: ask for a URL, PUT the bytes somewhere that
 * is not this API, poll until the variants exist, and remove. Each can fail on
 * its own and each failure means something different to the writer.
 */

const file = { name: 'hero.png', size: 400_000, type: 'image/png' } as File;

/** What a stubbed `fetch` was asked for. The mock's own tuple type is empty. */
const sent = (mock: { mock: { calls: unknown[] } }) =>
	mock.mock.calls as unknown as [string, RequestInit][];

const ok = (body: unknown, status = 200) =>
	new Response(JSON.stringify(body), { status, headers: { 'content-type': 'application/json' } });

test('asks for a URL with everything the policy needs declared', async () => {
	const fetchFn = vi.fn(async () =>
		ok({ data: { media_id: 'm-1', upload_url: 'https://gcs/put' } }, 201)
	);

	const result = await beginUpload(
		{ postId: 'post-1', file, altText: 'A hexagonal diagram' },
		fetchFn as unknown as typeof fetch
	);

	expect(result).toEqual({ ok: true, mediaId: 'm-1', uploadUrl: 'https://gcs/put' });

	const [, init] = sent(fetchFn)[0];
	const body = JSON.parse(String(init.body));
	expect(body).toMatchObject({
		attachment_target: 'blog_post',
		attachment_target_id: 'post-1',
		role: 'cover',
		file_name: 'hero.png',
		file_size_bytes: 400_000,
		mime_type: 'image/png',
		alt_text: 'A hexagonal diagram'
	});
});

test('a rejected upload keeps its code, so the drop zone can say which rule', async () => {
	const fetchFn = vi.fn(async () => ok({ error: { code: 'FILE_TOO_LARGE' } }, 400));

	const result = await beginUpload(
		{ postId: 'post-1', file, altText: 'x' },
		fetchFn as unknown as typeof fetch
	);

	expect(result).toMatchObject({ ok: false, kind: 'fileRejected' });
});

test('a response with no media id is a failure, not a half-made upload', async () => {
	// There would be nothing to PUT to and nothing to poll.
	const fetchFn = vi.fn(async () => ok({ data: { upload_url: 'https://gcs/put' } }, 201));

	const result = await beginUpload(
		{ postId: 'post-1', file, altText: 'x' },
		fetchFn as unknown as typeof fetch
	);

	expect(result.ok).toBe(false);
});

/** A minimal stand-in for the one browser API `fetch` cannot replace. */
function fakeXhr() {
	const xhr = {
		upload: {} as {
			onprogress?: (e: { lengthComputable: boolean; loaded: number; total: number }) => void;
		},
		status: 200,
		onload: undefined as (() => void) | undefined,
		onerror: undefined as (() => void) | undefined,
		open: vi.fn(),
		send: vi.fn(),
		setRequestHeader: vi.fn()
	};
	return xhr;
}

test('reports real progress while the bytes go up', async () => {
	// §03: "uploading shows determinate progress (the only honest bar)". `fetch`
	// cannot report request progress at all, so this one call is an
	// XMLHttpRequest and the bar is measured rather than animated.
	const xhr = fakeXhr();
	const seen: number[] = [];

	const done = uploadBytes('https://gcs/put', file, {
		onprogress: (fraction) => seen.push(fraction),
		open: () => xhr as unknown as XMLHttpRequest
	});

	xhr.upload.onprogress?.({ lengthComputable: true, loaded: 50, total: 200 });
	xhr.upload.onprogress?.({ lengthComputable: true, loaded: 200, total: 200 });
	xhr.onload?.();

	await expect(done).resolves.toMatchObject({ ok: true });
	expect(seen).toEqual([0.25, 1]);
});

test('a progress event that cannot be measured does not invent a number', async () => {
	const xhr = fakeXhr();
	const seen: number[] = [];

	const done = uploadBytes('https://gcs/put', file, {
		onprogress: (fraction) => seen.push(fraction),
		open: () => xhr as unknown as XMLHttpRequest
	});

	xhr.upload.onprogress?.({ lengthComputable: false, loaded: 50, total: 0 });
	xhr.onload?.();

	await done;
	expect(seen).toEqual([]);
});

test('a storage refusal is a failure the card can show', async () => {
	const xhr = fakeXhr();
	xhr.status = 403;

	const done = uploadBytes('https://gcs/put', file, {
		open: () => xhr as unknown as XMLHttpRequest
	});

	xhr.onload?.();

	await expect(done).resolves.toMatchObject({ ok: false });
});

test('reads the processing state, which is what says the file is usable', async () => {
	const fetchFn = vi.fn(async () => ok({ data: { status: 'processing' } }));

	await expect(readState('m-1', fetchFn as unknown as typeof fetch)).resolves.toBe('processing');
});

test('a media row that has gone reads as failed rather than as forever pending', async () => {
	// 404 means deleted or never there. Polling it forever would leave a
	// spinner on the rail with nothing behind it.
	const fetchFn = vi.fn(async () => ok({ error: { code: 'NOT_FOUND' } }, 404));

	await expect(readState('m-1', fetchFn as unknown as typeof fetch)).resolves.toBe('failed');
});

test('removing the cover deletes the media, not just the reference', async () => {
	const fetchFn = vi.fn(async () => ok({ data: null }));

	await expect(removeCover('m-1', fetchFn as unknown as typeof fetch)).resolves.toMatchObject({
		ok: true
	});
	const [url, init] = sent(fetchFn)[0];
	expect(url).toBe('/api/media/m-1');
	expect(init.method).toBe('DELETE');
});

/**
 * Correcting a description after the fact.
 *
 * `PATCH /api/media/{id}` was added because "a missing or wrong alt text was a
 * permanent accessibility defect". A card that never offers the fix would
 * quietly re-create the defect the endpoint removed.
 */

test('sends only the field being corrected', async () => {
	// PATCH changes what is present. Sending `caption: null` alongside would
	// clear a caption nobody asked to touch.
	const fetchFn = vi.fn(async () => ok({ data: null }));

	await correctAltText('m-1', 'A hexagonal diagram of the API', fetchFn as unknown as typeof fetch);

	const [url, init] = sent(fetchFn)[0];
	expect(url).toBe('/api/media/m-1');
	expect(init.method).toBe('PATCH');
	expect(JSON.parse(String(init.body))).toEqual({ alt_text: 'A hexagonal diagram of the API' });
});

test('a refused correction says so rather than looking saved', async () => {
	const fetchFn = vi.fn(async () => ok({ error: { code: 'NOT_FOUND' } }, 404));

	await expect(
		correctAltText('m-1', 'x', fetchFn as unknown as typeof fetch)
	).resolves.toMatchObject({ ok: false });
});
