<script lang="ts">
	import { X } from '@lucide/svelte';
	import type { Snippet } from 'svelte';

	/**
	 * A confirmation for something a person cannot see happen.
	 *
	 * §06 draws the boundary: an autosaving screen reports itself in one fixed
	 * place near the title, and this is not that. A toast is for the consequence
	 * that landed somewhere else — a post now live at a public address.
	 *
	 * Eight seconds, and the clock stops while the pointer or the keyboard is on
	 * it: a toast that vanishes on the way to its own link is the reason people
	 * stop trusting them.
	 *
	 * Polite. Nothing here has been lost, so nothing here interrupts.
	 */
	let {
		message,
		action,
		onclose,
		/** §08's number. */
		after = 8000
	}: {
		message: string;
		/** What to do next — a link to the thing that happened, usually. */
		action?: Snippet;
		onclose: () => void;
		after?: number;
	} = $props();

	let held = $state(false);
	let timer: ReturnType<typeof setTimeout> | undefined;

	$effect(() => {
		if (held) {
			clearTimeout(timer);
			return;
		}

		timer = setTimeout(onclose, after);
		return () => clearTimeout(timer);
	});
</script>

<!--
	The pointer and focus handlers only hold the timer open while someone is
	reading. They add no behaviour of their own, and the toast is reachable and
	dismissible without them.
-->
<div
	role="status"
	onmouseenter={() => (held = true)}
	onmouseleave={() => (held = false)}
	onfocusin={() => (held = true)}
	onfocusout={() => (held = false)}
	class="flex items-center gap-3 rounded-xl border border-arch-line bg-arch-surface
	       px-4 py-3 shadow-lg"
>
	<p class="text-[13px] text-arch-headline">{message}</p>

	{#if action}
		{@render action()}
	{/if}

	<button
		type="button"
		aria-label="Dismiss"
		onclick={onclose}
		class="-mr-1 ml-auto flex size-9 items-center justify-center rounded-lg text-arch-muted
		       transition-colors hover:text-arch-headline"
	>
		<X size={15} aria-hidden="true" />
	</button>
</div>
