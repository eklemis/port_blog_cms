<script lang="ts">
	/**
	 * What a list looks like before it arrives — CollectionState's loading
	 * state (20:2).
	 *
	 * Bars in the same card the other three states use, never a centred
	 * spinner: a spinner says "wait", a skeleton says something is coming and
	 * roughly how much. The four lengths are the frame's own, so the rows read
	 * as rows rather than as a progress bar.
	 *
	 * The shapes are decoration and hidden from assistive technology — a screen
	 * reader walking them would read four empty boxes. The one thing it should
	 * hear is the label, once.
	 */
	let {
		label,
		/** Four, as the frame draws. */
		rows = 4
	}: {
		label: string;
		rows?: number;
	} = $props();

	/** 317, 253, 296 and 211px — CollectionState 20:3 to 20:6. */
	const LENGTHS = [317, 253, 296, 211];

	const shapes = $derived(Array.from({ length: rows }, (_, index) => LENGTHS[index % 4]));
</script>

<div
	class="flex min-h-[226px] flex-col items-start gap-3 rounded-xl border border-arch-line
	       bg-arch-surface p-6"
>
	<div role="status" class="sr-only">{label}</div>

	{#each shapes as length, index (index)}
		<div
			data-skeleton-row
			aria-hidden="true"
			class="h-3.5 max-w-full rounded bg-arch-surface-2"
			style:width="{length}px"
		></div>
	{/each}
</div>
