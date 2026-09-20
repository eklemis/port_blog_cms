import { expect, test, vi } from 'vitest';
import { beginUpload, uploadBytes } from './upload';

/**
 * Starting an upload, for whatever it is attached to.
 *
 * Two features need this — a post's cover and a project's screenshots — so it
 * lives below both rather than being written twice. What differs between them
 * is three values (target, id, role), which is exactly what the caller passes.
 *
 * ADR 0008 fixed the wire format: `"screenshot"` and `"blog_post"`, lowercase
 * and snake_case, where capitalised forms used to be sent. "Any client sending
 * the capitalized forms breaks."
 */

const file = { name: 'hero.png', size: 400_000, type: 'image/png' } as File;

const ok = (body: unknown, status = 200) =>
	new Response(JSON.stringify(body), { status, headers: { 'content-type': 'application/json' } });

const sent = (mock: { mock: { calls: unknown[] } }) =>
	mock.mock.calls as unknown as [string, RequestInit][];

test('declares the attachment the caller named, in the wire forms the API takes', async () => {
	const fetchFn = vi.fn(async () =>
		ok({ data: { media_id: 'm-1', upload_url: 'https://gcs' } }, 201)
	);

	await beginUpload(
		{ target: 'project', targetId: 'p-1', role: 'screenshot', file, altText: 'The posts list' },
		fetchFn as unknown as typeof fetch
	);

	expect(JSON.parse(String(sent(fetchFn)[0][1].body))).toMatchObject({
		attachment_target: 'project',
		attachment_target_id: 'p-1',
		role: 'screenshot',
		file_name: 'hero.png',
		file_size_bytes: 400_000,
		mime_type: 'image/png',
		alt_text: 'The posts list'
	});
});

test('a cover on a post is the same call with three values changed', async () => {
	const fetchFn = vi.fn(async () =>
		ok({ data: { media_id: 'm-1', upload_url: 'https://gcs' } }, 201)
	);

	await beginUpload(
		{ target: 'blog_post', targetId: 'post-1', role: 'cover', file, altText: 'A diagram' },
		fetchFn as unknown as typeof fetch
	);

	expect(JSON.parse(String(sent(fetchFn)[0][1].body))).toMatchObject({
		attachment_target: 'blog_post',
		role: 'cover'
	});
});

test('a position is sent only when the caller has one to give', async () => {
	// A cover has no position; a screenshot joining a gallery does.
	const fetchFn = vi.fn(async () =>
		ok({ data: { media_id: 'm-1', upload_url: 'https://gcs' } }, 201)
	);

	await beginUpload(
		{ target: 'project', targetId: 'p-1', role: 'screenshot', file, altText: 'x', position: 3 },
		fetchFn as unknown as typeof fetch
	);

	expect(JSON.parse(String(sent(fetchFn)[0][1].body)).position).toBe(3);
});

test('a rejected upload keeps its code, so the drop zone can name the rule', async () => {
	const fetchFn = vi.fn(async () => ok({ error: { code: 'FILE_TOO_LARGE' } }, 400));

	await expect(
		beginUpload(
			{ target: 'project', targetId: 'p-1', role: 'screenshot', file, altText: 'x' },
			fetchFn as unknown as typeof fetch
		)
	).resolves.toMatchObject({ ok: false, kind: 'fileRejected' });
});

test('a response missing either half is a failure, not a half-made upload', async () => {
	const fetchFn = vi.fn(async () => ok({ data: { upload_url: 'https://gcs' } }, 201));

	await expect(
		beginUpload(
			{ target: 'project', targetId: 'p-1', role: 'screenshot', file, altText: 'x' },
			fetchFn as unknown as typeof fetch
		)
	).resolves.toMatchObject({ ok: false });
});

/** A stand-in for the one browser API `fetch` cannot replace. */
function fakeXhr() {
	return {
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
}

test('progress is measured, because a bar over a real transfer must be real', async () => {
	const xhr = fakeXhr();
	const seen: number[] = [];

	const done = uploadBytes('https://gcs', file, {
		onprogress: (fraction) => seen.push(fraction),
		open: () => xhr as unknown as XMLHttpRequest
	});

	xhr.upload.onprogress?.({ lengthComputable: true, loaded: 50, total: 200 });
	xhr.onload?.();

	await expect(done).resolves.toMatchObject({ ok: true });
	expect(seen).toEqual([0.25]);
});

test('storage refusing the bytes is a failure the card can show', async () => {
	const xhr = fakeXhr();
	xhr.status = 403;

	const done = uploadBytes('https://gcs', file, { open: () => xhr as unknown as XMLHttpRequest });
	xhr.onload?.();

	await expect(done).resolves.toMatchObject({ ok: false });
});
