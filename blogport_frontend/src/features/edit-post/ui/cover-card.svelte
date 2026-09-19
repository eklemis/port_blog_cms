<script lang="ts">
	import { StatusPill, Button, Field } from '$lib/shared/ui';
	import { checkDeclared, checkDimensions, pillFor, type Rejection } from '$lib/entities/media';
	import { beginUpload, removeCover, uploadBytes } from '../api/cover';
	import type { MediaState } from '$lib/entities/media';

	/**
	 * The editor rail's Cover image card — Screen / Post editor 32:999.
	 *
	 * The frame draws one state, `processing`. §03's MediaTile row specifies the
	 * rest: "uploading · pending · processing · ready · failed — uploading shows
	 * determinate progress (the only honest bar); pending and processing are
	 * indeterminate. failed persists until dismissed — a silently vanishing
	 * upload is worse than a visible failure."
	 *
	 * **One line of the frame is not built, deliberately.** 32:1005 reads "Alt
	 * text is set once and cannot be edited later." That was true and is not:
	 * `PATCH /api/media/{id}` exists for exactly this reason — "alt text, caption
	 * and position are set at upload and were not editable, so a missing or wrong
	 * alt text was a permanent accessibility defect" — and §03 now says "editable
	 * afterwards via PATCH, but treat the picker as the real moment." Shipping
	 * the sentence would talk writers out of fixing bad alt text, which is the
	 * opposite of what it is there to do. The card keeps the urgency and drops
	 * the false half; the wording is with the designer.
	 *
	 * Alt text goes up *with* the upload request rather than after it, because
	 * §03 is right that nobody comes back to it.
	 */
	type Cover = { media_id: string; status: MediaState; alt_text: string };

	let {
		postId,
		cover = null,
		coverSrc = null,
		onchanged = () => {},
		/**
		 * The three browser capabilities a test has to stand in for: the network,
		 * the byte transfer, and the image decoder. Real ones by default.
		 */
		fetchFn = undefined,
		upload = uploadBytes,
		measure = decode
	}: {
		postId: string;
		cover?: Cover | null;
		/** A signed read URL, resolved with the page. Short-lived by design. */
		coverSrc?: string | null;
		/** The cover changed; the page reloads rather than this guessing. */
		onchanged?: () => void;
		fetchFn?: typeof globalThis.fetch;
		upload?: typeof uploadBytes;
		measure?: (file: File) => Promise<{ width: number; height: number }>;
	} = $props();

	/** The only place the browser can measure an image: after decoding it. */
	async function decode(file: File): Promise<{ width: number; height: number }> {
		const bitmap = await createImageBitmap(file);
		const size = { width: bitmap.width, height: bitmap.height };
		bitmap.close();
		return size;
	}

	let chosen = $state<File | null>(null);
	let altText = $state('');
	let rejection = $state<Rejection | null>(null);
	let sending = $state(false);
	let fraction = $state(0);

	const pill = $derived(cover ? pillFor(cover.status) : null);
	const ready = $derived(cover?.status === 'ready');
	const inFlight = $derived(sending || (cover !== null && !ready && cover.status !== 'failed'));

	const fieldId = $props.id();

	async function pick(event: Event) {
		const file = (event.currentTarget as HTMLInputElement).files?.[0] ?? null;

		chosen = null;
		rejection = null;
		if (!file) return;

		// Size and type from the handle; the edges only after a decode. All three
		// before anything leaves the browser — §03, and the only chance, since
		// the bytes never pass through the API.
		const declared = checkDeclared(file);
		if (declared) return (rejection = declared);

		try {
			const { width, height } = await measure(file);
			const edges = checkDimensions(width, height);
			if (edges) return (rejection = edges);
		} catch {
			// Undecodable is not an image, whatever it says it is.
			return (rejection = { code: 'INVALID_MIME_TYPE', message: 'That file is not an image.' });
		}

		chosen = file;
	}

	async function send() {
		if (!chosen || !altText.trim()) return;

		sending = true;
		fraction = 0;

		const started = await beginUpload({ postId, file: chosen, altText: altText.trim() }, fetchFn);

		if (!started.ok) {
			sending = false;
			rejection = { code: 'INVALID_MIME_TYPE', message: started.message };
			return;
		}

		await upload(started.uploadUrl, chosen, {
			onprogress: (value) => (fraction = value)
		});

		// Whatever happened to the bytes, the row exists and the page is the one
		// that knows how to read its state.
		sending = false;
		chosen = null;
		altText = '';
		onchanged();
	}

	async function drop() {
		if (!cover) return;

		await removeCover(cover.media_id, fetchFn);
		onchanged();
	}
</script>

<section
	aria-label="Cover image"
	class="flex flex-col gap-2.5 rounded-xl border border-arch-line bg-arch-surface px-4 py-[15px]"
>
	<h2 class="font-mono text-[9px] font-normal tracking-[0.9px] text-arch-muted uppercase">
		Cover image
	</h2>

	<div
		class="flex h-[132px] items-center justify-center overflow-hidden rounded-lg bg-arch-surface-2"
	>
		{#if ready && coverSrc}
			<img src={coverSrc} alt={cover?.alt_text ?? ''} class="size-full object-cover" />
		{:else if sending}
			<!-- The determinate bar. A real fraction of real bytes. -->
			<div
				role="progressbar"
				aria-label="Uploading"
				aria-valuenow={Math.round(fraction * 100)}
				aria-valuemin="0"
				aria-valuemax="100"
				class="h-1.5 w-[70%] overflow-hidden rounded-full bg-arch-line"
			>
				<div class="h-full bg-arch-accent-ink" style="width: {Math.round(fraction * 100)}%"></div>
			</div>
		{:else if pill}
			<StatusPill tone={pill.tone} label={pill.label} />
		{:else}
			<label
				class="cursor-pointer rounded-lg border border-arch-line-control px-3 py-1.5 text-[11.5px]
				       text-arch-headline hover:bg-arch-surface"
			>
				Choose image
				<input
					type="file"
					accept="image/jpeg,image/png,image/webp"
					class="sr-only"
					onchange={pick}
				/>
			</label>
		{/if}
	</div>

	<!-- The state changes while nobody is looking at this corner, so it is
	     announced once it settles rather than on every poll. -->
	<p class="sr-only" aria-live="polite">
		{#if ready}Cover image ready.{:else if cover?.status === 'failed'}Cover image failed.{/if}
	</p>

	{#if rejection}
		<p class="text-[10.5px] text-st-danger" role="status">{rejection.message}</p>
	{/if}

	{#if chosen && !inFlight}
		<Field
			id={fieldId}
			label="Alt text"
			bind:value={altText}
			placeholder="A hexagonal diagram of the API"
			help="Describes the image for anyone who cannot see it."
		/>
		<div class="flex gap-2">
			<Button label="Upload" disabled={!altText.trim()} onclick={send} />
		</div>
	{/if}

	{#if cover && !sending}
		<div class="flex items-center justify-between gap-3">
			<p class="min-w-0 truncate text-[10.5px] text-arch-muted">{cover.alt_text}</p>
			<Button kind="ghost" label="Remove" onclick={drop} />
		</div>
	{/if}

	{#if !cover && !chosen}
		<p class="text-[10.5px] text-arch-muted">Alt text is set as you upload.</p>
	{/if}
</section>
