<script lang="ts">
	import type { Snippet } from 'svelte';
	import { Ellipsis } from '@lucide/svelte';

	/**
	 * The ⋯ menu at the end of a row, and at the end of the editor's top bar.
	 *
	 * §06 puts Archive behind this rather than on the row: archiving is one
	 * click, and one click is exactly what a row-level button gets by accident.
	 * Not on phone cards — "a swipe is undiscoverable and a 32px menu on a card
	 * invites the wrong tap".
	 *
	 * Escape closes it and returns focus to the trigger; so does choosing
	 * something, so the row is never left covered.
	 */
	let {
		label,
		items
	}: {
		/** Names the row it belongs to: "More for Building a CMS". */
		label: string;
		/**
		 * `role="menuitem"` buttons. They are handed `close` and call it when
		 * chosen — a click handler on the menu box itself would be a click
		 * target with no keyboard equivalent.
		 */
		items: Snippet<[() => void]>;
	} = $props();

	let open = $state(false);
	let trigger = $state<HTMLButtonElement>();

	function close({ toTrigger = false } = {}) {
		open = false;
		if (toTrigger) trigger?.focus();
	}
</script>

<svelte:window
	onkeydown={(event) => {
		if (open && event.key === 'Escape') close({ toTrigger: true });
	}}
/>

<div class="relative">
	<button
		bind:this={trigger}
		type="button"
		aria-label={label}
		aria-haspopup="menu"
		aria-expanded={open}
		onclick={() => (open = !open)}
		class="flex size-8 items-center justify-center rounded-lg text-arch-muted
		       transition-colors hover:bg-arch-surface-2 hover:text-arch-headline"
	>
		<Ellipsis size={17} aria-hidden="true" />
	</button>

	{#if open}
		<!-- Anything else on the page closes it; the menu's own clicks do not
		     reach here, and choosing an item closes it below. -->
		<button
			type="button"
			tabindex="-1"
			aria-hidden="true"
			onclick={() => close()}
			class="fixed inset-0 z-10 cursor-default"
		></button>
		<div
			role="menu"
			tabindex="-1"
			class="absolute right-0 z-20 mt-1 flex min-w-[168px] flex-col rounded-xl border
			       border-arch-line bg-arch-surface py-1.5 shadow-lg"
		>
			{@render items(() => close({ toTrigger: true }))}
		</div>
	{/if}
</div>
