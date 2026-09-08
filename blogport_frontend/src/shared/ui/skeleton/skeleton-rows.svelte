<script lang="ts">
	/**
	 * What a list looks like before it arrives.
	 *
	 * Rows shaped like real rows, never a centred spinner: a spinner says
	 * "wait", a skeleton says "a table is coming and it is about this big".
	 *
	 * The shapes are decoration and are hidden from assistive technology — a
	 * screen reader walking them would read a dozen empty boxes. The one thing
	 * it should hear is the label, once.
	 */
	let {
		label,
		/** Six, because that is what a page of rows looks like. */
		rows = 6
	}: {
		label: string;
		rows?: number;
	} = $props();

	const shapes = $derived(Array.from({ length: rows }, (_, index) => index));
</script>

<div class="overflow-hidden rounded-xl border border-arch-line bg-arch-surface">
	<div role="status" class="sr-only">{label}</div>

	{#each shapes as row (row)}
		<div
			data-skeleton-row
			aria-hidden="true"
			class="flex h-[46px] items-center gap-4 border-b border-arch-line px-[18px] last:border-b-0"
		>
			<div class="h-3 w-[46%] rounded bg-arch-surface-2"></div>
			<div class="h-3 w-16 rounded bg-arch-surface-2"></div>
			<div class="ml-auto h-3 w-20 rounded bg-arch-surface-2"></div>
		</div>
	{/each}
</div>
