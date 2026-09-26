<script lang="ts" generics="T">
	import type { Snippet } from 'svelte';
	import { untrack } from 'svelte';

	/**
	 * One of the résumé's repeatable collections — Screen / CV builder 69:2.
	 *
	 * The frame draws these four as a name, a count and "+ Add"; **no frame
	 * shows one open**, so the shape is borrowed from the experience list on the
	 * same screen rather than invented fresh. Its rule is stated there and holds
	 * here: rows collapse to a summary and one opens at a time.
	 *
	 * The caller owns what a row *is* — the fields, the blank, the summary. This
	 * owns only the list behaviour, which is the part that would otherwise be
	 * written four times.
	 *
	 * Every change reports the whole list, because every collection on
	 * `PatchCVRequest` is a `ReplaceOp`: replaced wholesale or left alone.
	 */
	let {
		label,
		items,
		blank,
		summary,
		fields,
		onchange
	}: {
		label: string;
		items: T[];
		/** A new, empty row. */
		blank: () => T;
		/** The one line a collapsed row shows. */
		summary: (item: T) => string;
		/** The row's own fields, given the row and a way to change it. */
		fields: Snippet<[T, (patch: Partial<T>) => void]>;
		onchange: (items: T[]) => void;
	} = $props();

	// A seed: the list is being edited here, and a loader that re-ran would
	// otherwise close the row somebody was typing in.
	let list = $state<T[]>(untrack(() => items.map((item) => ({ ...item }))));
	let open = $state<number | null>(null);

	function commit(next: T[]) {
		list = next;
		onchange(next);
	}

	function update(index: number, patch: Partial<T>) {
		// Spread rather than rebuilt, so any field this form does not draw
		// survives an edit that never touched it.
		commit(list.map((item, at) => (at === index ? { ...item, ...patch } : item)));
	}

	function add() {
		open = list.length;
		commit([...list, blank()]);
	}
</script>

<section
	aria-label={label}
	class="flex flex-col gap-2.5 rounded-xl border border-arch-line bg-arch-surface p-5"
>
	<div class="flex items-center justify-between gap-3">
		<h2 class="font-mono text-[9px] font-normal tracking-[0.9px] text-arch-muted uppercase">
			{label}
		</h2>
		<button
			type="button"
			aria-label="Add to {label.toLowerCase()}"
			onclick={add}
			class="text-[11px] text-arch-accent-ink hover:underline"
		>
			+ Add
		</button>
	</div>

	{#if list.length}
		<ul class="flex list-none flex-col gap-2 p-0">
			{#each list as item, index (index)}
				{@const line = summary(item)}
				<li class="rounded-lg border border-arch-line">
					{#if open === index}
						<div class="flex flex-col gap-2.5 p-3">
							{@render fields(item, (patch) => update(index, patch))}
							<div class="flex items-center justify-between gap-2">
								<button
									type="button"
									onclick={() => (open = null)}
									class="text-[11px] text-arch-muted hover:text-arch-headline"
								>
									Done
								</button>
								<button
									type="button"
									aria-label="Remove {line}"
									onclick={() => {
										open = null;
										commit(list.filter((_, at) => at !== index));
									}}
									class="text-[11px] text-st-danger hover:underline"
								>
									Remove
								</button>
							</div>
						</div>
					{:else}
						<div class="flex items-center justify-between gap-2 px-3 py-2">
							<span class="min-w-0 truncate text-[11.5px] text-arch-headline">{line}</span>
							<button
								type="button"
								aria-label="Edit {line}"
								onclick={() => (open = index)}
								class="shrink-0 text-[11px] text-arch-muted hover:text-arch-headline"
							>
								Edit
							</button>
						</div>
					{/if}
				</li>
			{/each}
		</ul>
	{:else}
		<p class="text-[11.5px] text-arch-muted">
			{list.length}
			{list.length === 1 ? 'entry' : 'entries'}
		</p>
	{/if}
</section>
