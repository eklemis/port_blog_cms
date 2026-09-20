<script lang="ts">
	import { untrack } from 'svelte';
	import { ChevronDown, ChevronUp, GripVertical } from '@lucide/svelte';
	import {
		beginUpload,
		checkDeclared,
		checkDimensions,
		moved,
		patchMedia,
		repositioned,
		uploadBytes,
		type Rejection
	} from '$lib/entities/media';
	import { Button, Field } from '$lib/shared/ui';

	/**
	 * The project editor's screenshots card — Screen / Project editor 70:257.
	 *
	 * Reordering is what `PatchMediaRequest.position` unblocked. The endpoint
	 * says why it was added: "a gallery could not be reordered without
	 * re-uploading every image."
	 *
	 * **Drag is not the only way.** §04 is explicit — "No drag-only interaction —
	 * media reordering offers move up / move down alongside the drag" — so the
	 * handle is an affordance and the buttons are the mechanism. Both go through
	 * the same arithmetic, because a keyboard reorder that lands somewhere a drag
	 * would not is two features wearing one name.
	 *
	 * Optimistic: the list moves at once and the requests follow. A refusal puts
	 * it back, because a rail still showing an order the server rejected is worse
	 * than one that flickers.
	 *
	 * Adding one is the same flow as a post's cover, through the same entity call
	 * with three values changed: target `project`, role `screenshot`, and a
	 * position at the end of the gallery. ADR 0008 fixed those spellings —
	 * lowercase and snake_case — and says plainly that "any client sending the
	 * capitalized forms breaks".
	 *
	 * All three policy checks run before anything leaves the browser, because
	 * the bytes go straight to storage and the API never sees them.
	 */
	type Shot = { media_id: string; original_filename: string; src?: string | null };

	let {
		screenshots,
		projectId = undefined,
		onchanged = () => {},
		fetchFn = undefined,
		upload = uploadBytes,
		measure = decode
	}: {
		/** In display order — `position` ascending, as the server returned them. */
		screenshots: Shot[];
		/** Whose gallery this is. Without one there is nothing to attach to. */
		projectId?: string;
		/** The gallery changed and the server took it. */
		onchanged?: () => void;
		fetchFn?: typeof globalThis.fetch;
		/** The two browser capabilities a test has to stand in for. */
		upload?: typeof uploadBytes;
		measure?: (file: File) => Promise<{ width: number; height: number }>;
	} = $props();

	/** The only place an image can be measured: after the browser decodes it. */
	async function decode(file: File): Promise<{ width: number; height: number }> {
		const bitmap = await createImageBitmap(file);
		const size = { width: bitmap.width, height: bitmap.height };
		bitmap.close();
		return size;
	}

	// A seed: this list is the one being dragged, and a loader that re-ran
	// mid-gesture would otherwise yank it out from under the pointer.
	let order = $state<Shot[]>(untrack(() => [...screenshots]));
	let failure = $state<string | null>(null);
	let announcement = $state('');
	let dragging = $state<number | null>(null);

	const describedId = $props.id();
	const altId = `${describedId}-alt`;

	let chosen = $state<File | null>(null);
	let altText = $state('');
	let rejection = $state<Rejection | null>(null);
	let sending = $state(false);

	async function pick(event: Event) {
		const file = (event.currentTarget as HTMLInputElement).files?.[0] ?? null;

		chosen = null;
		rejection = null;
		if (!file) return;

		const declared = checkDeclared(file);
		if (declared) return (rejection = declared);

		try {
			const { width, height } = await measure(file);
			const edges = checkDimensions(width, height);
			if (edges) return (rejection = edges);
		} catch {
			// Undecodable is not an image, whatever it claims to be.
			return (rejection = { code: 'INVALID_MIME_TYPE', message: 'That file is not an image.' });
		}

		chosen = file;
	}

	async function add() {
		if (!chosen || !projectId || !altText.trim()) return;

		sending = true;
		failure = null;

		const started = await beginUpload(
			{
				target: 'project',
				targetId: projectId,
				role: 'screenshot',
				file: chosen,
				altText: altText.trim(),
				// The end of the gallery. Reordering is a separate gesture, and
				// dropping a new image into the middle is not one anybody asked for.
				position: order.length
			},
			fetchFn
		);

		if (!started.ok) {
			sending = false;
			rejection = { code: 'INVALID_MIME_TYPE', message: started.message };
			return;
		}

		await upload(started.uploadUrl, chosen);

		// The row's filename and processing state are the server's to report.
		sending = false;
		chosen = null;
		altText = '';
		onchanged();
	}

	async function reorder(from: number, to: number) {
		const before = order;
		const after = moved(before, from, to);

		if (after === before || after.every((row, index) => row.media_id === before[index].media_id)) {
			return;
		}

		order = after;
		failure = null;

		const index = after.findIndex((row) => row.media_id === before[from].media_id);
		announcement = `${before[from].original_filename} moved to position ${index + 1} of ${after.length}.`;

		// Only what moved. Each row is its own request, so sending the whole
		// list would be failures for rows that never changed.
		const changes = repositioned(before, after);
		const results = await Promise.all(
			changes.map((change) => patchMedia(change.media_id, { position: change.position }, fetchFn))
		);

		if (results.some((result) => !result.ok)) {
			order = before;
			announcement = '';
			failure = 'That order didn’t save. The list is as it was.';
			return;
		}

		onchanged();
	}
</script>

<section
	aria-label="Screenshots"
	class="flex w-full flex-col gap-2.5 rounded-xl border border-arch-line bg-arch-surface p-5"
>
	<div class="flex items-center justify-between gap-3">
		<h2 class="font-mono text-[9px] font-normal tracking-[0.9px] text-arch-muted uppercase">
			Screenshots
		</h2>

		{#if projectId}
			<label
				class="cursor-pointer text-[11px] text-arch-accent-ink hover:underline"
				class:opacity-40={sending}
			>
				+ Upload
				<input
					type="file"
					accept="image/jpeg,image/png,image/webp"
					class="sr-only"
					disabled={sending}
					onchange={pick}
				/>
			</label>
		{/if}
	</div>

	{#if rejection}
		<p role="status" class="text-[10.5px] text-st-danger">{rejection.message}</p>
	{/if}

	{#if chosen}
		<div class="flex flex-col gap-2 rounded-lg bg-arch-surface-2 p-3">
			<p class="truncate font-mono text-[11px] text-arch-headline">{chosen.name}</p>
			<Field
				id={altId}
				label="Alt text"
				bind:value={altText}
				help="Describes the image for anyone who cannot see it."
			/>
			<div class="flex gap-2">
				<Button label="Add" disabled={!altText.trim()} loading={sending} onclick={add} />
				<Button
					kind="ghost"
					label="Cancel"
					onclick={() => {
						chosen = null;
						altText = '';
					}}
				/>
			</div>
		</div>
	{/if}

	{#if order.length}
		<ul class="flex list-none flex-col gap-2.5 p-0">
			{#each order as shot, index (shot.media_id)}
				<li
					draggable={order.length > 1}
					ondragstart={() => (dragging = index)}
					ondragover={(event) => event.preventDefault()}
					ondrop={(event) => {
						event.preventDefault();
						if (dragging !== null) reorder(dragging, index);
						dragging = null;
					}}
					class="flex h-11 items-center gap-2.5 rounded-lg border border-arch-line px-2.5 py-2"
				>
					<!-- The frame's ⠿. An affordance for the pointer; the buttons
					     below are what actually moves a row. -->
					<GripVertical size={13} class="shrink-0 text-arch-muted" aria-hidden="true" />

					<div class="h-[26px] w-[38px] shrink-0 overflow-hidden rounded bg-arch-surface-2">
						{#if shot.src}
							<img src={shot.src} alt="" class="size-full object-cover" />
						{/if}
					</div>

					<span
						data-filename
						class="min-w-0 flex-1 truncate font-mono text-[11px] text-arch-headline"
					>
						{shot.original_filename}
					</span>

					{#if order.length > 1}
						<div class="flex shrink-0 items-center">
							<button
								type="button"
								aria-label="Move {shot.original_filename} up"
								disabled={index === 0}
								aria-describedby={index === 0 ? describedId : undefined}
								onclick={() => reorder(index, index - 1)}
								class="rounded p-1 text-arch-muted hover:text-arch-headline disabled:opacity-40"
							>
								<ChevronUp size={14} aria-hidden="true" />
							</button>
							<button
								type="button"
								aria-label="Move {shot.original_filename} down"
								disabled={index === order.length - 1}
								aria-describedby={index === order.length - 1 ? describedId : undefined}
								onclick={() => reorder(index, index + 1)}
								class="rounded p-1 text-arch-muted hover:text-arch-headline disabled:opacity-40"
							>
								<ChevronDown size={14} aria-hidden="true" />
							</button>
						</div>
					{/if}
				</li>
			{/each}
		</ul>
	{:else}
		<p class="text-[11.5px] text-arch-muted">No screenshots yet.</p>
	{/if}

	<!-- §08: a disabled control carries its reason, never a bare attribute. -->
	<p id={describedId} class="sr-only">Already at the end of the list.</p>

	<!-- The row moves away from where the eye was, so the move is said as well
	     as shown. Once per move, not once per request. -->
	<p class="sr-only" aria-live="polite">{announcement}</p>

	{#if failure}
		<p role="status" class="text-[10.5px] text-st-danger">{failure}</p>
	{/if}

	<p class="text-[10.5px] text-arch-muted">
		Drag to reorder, or use the arrows. Order is what a reader sees on your public page.
	</p>
</section>
