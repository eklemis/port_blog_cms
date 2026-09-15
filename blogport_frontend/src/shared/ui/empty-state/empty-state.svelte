<script lang="ts">
	import type { Snippet } from 'svelte';

	/**
	 * The panel a list shows when it has no rows — Figma's CollectionState (20:23).
	 *
	 * One shape for the states §06 asks every collection to have: empty,
	 * filtered-empty and error. What differs between them is the sentence, the
	 * way out, and — for the error alone — the title's colour: "error never
	 * blames the user and always says their data is safe", and it is the only
	 * one of the three that is a failure.
	 *
	 * A 400px card centred in a 320px band, as every state frame draws it.
	 *
	 * The copy stays with the caller. "No posts match those filters" belongs to
	 * posts, and a shared component that tried to own it would end up with a
	 * resource name interpolated into a sentence nobody wrote.
	 */
	let {
		title,
		message,
		tone = 'neutral',
		action
	}: {
		title: string;
		message: string;
		/** `danger` for the error state only. */
		tone?: 'neutral' | 'danger';
		/** The one thing that fixes it. An error nobody can retry has none. */
		action?: Snippet;
	} = $props();
</script>

<div class="flex min-h-[320px] items-center justify-center">
	<div
		class="flex min-h-[226px] w-full max-w-[400px] flex-col items-center justify-center gap-3
		       rounded-xl border border-arch-line bg-arch-surface p-6 text-center"
	>
		<!-- A heading, so skipping by headings on an empty screen finds the reason. -->
		<h2
			class="font-display text-[17px] font-bold
			       {tone === 'danger' ? 'text-st-danger' : 'text-arch-headline'}"
		>
			{title}
		</h2>
		<p class="max-w-[320px] text-[12.5px] text-arch-muted">{message}</p>

		{#if action}
			{@render action()}
		{/if}
	</div>
</div>
