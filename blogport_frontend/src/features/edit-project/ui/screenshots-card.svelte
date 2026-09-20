<script lang="ts">
	import { untrack } from 'svelte';
	import { ChevronDown, ChevronUp, GripVertical } from '@lucide/svelte';
	import { moved, patchMedia, repositioned } from '$lib/entities/media';

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
	 * **Upload is not here yet.** The frame draws "+ Upload" beside the heading.
	 * The upload flow exists but lives in the post editor's feature slice, and
	 * slices in the same layer may not import each other — so it moves down to
	 * the media entity first, as its own change. Filed rather than duplicated.
	 */
	type Shot = { media_id: string; original_filename: string; src?: string | null };

	let {
		screenshots,
		onchanged = () => {},
		fetchFn = undefined
	}: {
		/** In display order — `position` ascending, as the server returned them. */
		screenshots: Shot[];
		/** The order changed and the server took it. */
		onchanged?: () => void;
		fetchFn?: typeof globalThis.fetch;
	} = $props();

	// A seed: this list is the one being dragged, and a loader that re-ran
	// mid-gesture would otherwise yank it out from under the pointer.
	let order = $state<Shot[]>(untrack(() => [...screenshots]));
	let failure = $state<string | null>(null);
	let announcement = $state('');
	let dragging = $state<number | null>(null);

	const describedId = $props.id();

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
	<h2 class="font-mono text-[9px] font-normal tracking-[0.9px] text-arch-muted uppercase">
		Screenshots
	</h2>

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
