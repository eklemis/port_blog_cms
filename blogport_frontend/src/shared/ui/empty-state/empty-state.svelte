<script lang="ts">
	import type { Snippet } from 'svelte';

	/**
	 * The panel a list shows when it has no rows.
	 *
	 * One shape for all three of the states §06 asks every collection to have —
	 * empty, filtered-empty and error. They are deliberately identical to look
	 * at: what has to differ between them is the sentence and the way out, and
	 * that is the pair worth guarding, not the box they sit in.
	 *
	 * The copy stays with the caller. "No posts match those filters" belongs to
	 * posts, and a shared component that tried to own it would end up with a
	 * resource name interpolated into a sentence nobody wrote.
	 */
	let {
		title,
		message,
		action
	}: {
		title: string;
		message: string;
		/** The one thing that fixes it. An error nobody can retry has none. */
		action?: Snippet;
	} = $props();
</script>

<div class="rounded-xl border border-arch-line bg-arch-surface p-8 text-center">
	<!-- A heading, so skipping by headings on an empty screen finds the reason. -->
	<h2 class="font-display text-[17px] font-extrabold text-arch-headline">{title}</h2>
	<p class="mt-2 text-[13px] text-arch-muted">{message}</p>

	{#if action}
		<div class="mt-4 flex justify-center">{@render action()}</div>
	{/if}
</div>
