<script lang="ts">
	import type { Snippet } from 'svelte';
	import { pageWindow } from '../model/pages';

	/**
	 * The count and the page numbers under a list — Screen / Posts list 11:117:
	 * "6 of 24 posts" on the left, "‹ 1 2 3 ›" in 12px mono on the right.
	 *
	 * The frame draws the numbers as text. Here each is a button with a 24px
	 * target (WCAG 2.2 AA 2.5.8) and a name that says it is a page, so a screen
	 * reader hears "Page 2, current page" rather than "2".
	 */
	let {
		shown,
		total,
		page,
		perPage,
		noun,
		after,
		onpage
	}: {
		/** Rows on this page. */
		shown: number;
		total: number;
		page: number;
		perPage: number;
		/** Plural, as the count reads: "posts", "applications". */
		noun: string;
		/** A destination beside the count — "View archive (3)" on the posts list. */
		after?: Snippet;
		onpage: (page: number) => void;
	} = $props();

	const last = $derived(Math.max(1, Math.ceil(total / perPage)));
	const slots = $derived(pageWindow(page, last));
</script>

<div class="flex items-center justify-between text-[12px] text-arch-muted">
	<div class="flex items-center gap-3">
		<p>{shown} of {total} {noun}</p>
		{#if after}{@render after()}{/if}
	</div>

	{#if last > 1}
		<nav aria-label="Pages" class="flex items-center gap-0.5 font-mono">
			<button
				type="button"
				aria-label="Previous page"
				disabled={page <= 1}
				onclick={() => onpage(page - 1)}
				class="flex min-h-6 min-w-6 items-center justify-center rounded disabled:opacity-40"
			>
				‹
			</button>
			{#each slots as slot, index (`${slot}-${index}`)}
				{#if slot === '…'}
					<span aria-hidden="true" class="px-1">…</span>
				{:else}
					<button
						type="button"
						aria-label="Page {slot}"
						aria-current={slot === page ? 'page' : undefined}
						onclick={() => onpage(slot)}
						class="flex min-h-6 min-w-6 items-center justify-center rounded
						       {slot === page ? 'font-semibold text-arch-headline' : 'hover:text-arch-headline'}"
					>
						{slot}
					</button>
				{/if}
			{/each}
			<button
				type="button"
				aria-label="Next page"
				disabled={page >= last}
				onclick={() => onpage(page + 1)}
				class="flex min-h-6 min-w-6 items-center justify-center rounded disabled:opacity-40"
			>
				›
			</button>
		</nav>
	{/if}
</div>
