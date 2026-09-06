<script lang="ts">
	import type { Snippet } from 'svelte';

	/**
	 * Button — the reference component for this codebase.
	 *
	 * Copy its shape when you build the other nine: runes for props, colour only
	 * through tokens, every state reachable, and the accessible name never moving.
	 * Design: Forms & Interaction Spec §04, Accessibility Spec §08.
	 */
	type Kind = 'primary' | 'secondary' | 'ghost' | 'danger';

	let {
		label,
		kind = 'primary',
		disabled = false,
		loading = false,
		disabledReason,
		icon,
		onclick
	}: {
		label: string;
		kind?: Kind;
		disabled?: boolean;
		loading?: boolean;
		/** Why it is disabled. A disabled control with no reason is a dead end. */
		disabledReason?: string;
		icon?: Snippet;
		onclick?: (event: MouseEvent) => void;
	} = $props();

	const KIND: Record<Kind, string> = {
		primary: 'bg-arch-accent text-arch-accent-on border-transparent',
		secondary:
			'bg-arch-surface text-arch-headline border-arch-line-control hover:bg-arch-surface-2',
		ghost: 'bg-transparent text-arch-headline border-transparent hover:bg-arch-surface-2',
		danger: 'bg-transparent text-st-danger border-st-danger hover:bg-st-danger/10'
	};

	// The label never changes while loading — a control whose accessible name
	// swaps to "Loading…" reads as a different control to a screen reader.
	const reasonId = $derived(
		disabled && disabledReason ? `${label.replace(/\W+/g, '-')}-reason` : undefined
	);
</script>

<button
	type="button"
	class="inline-flex min-h-11 items-center justify-center gap-2 rounded-lg border
	       px-4 py-2 text-sm font-semibold transition-colors
	       disabled:cursor-not-allowed disabled:opacity-55 {KIND[kind]}"
	disabled={disabled || loading}
	aria-busy={loading}
	aria-describedby={reasonId}
	{onclick}
>
	{#if loading}
		<!-- Suppressed under 400ms by the caller, per Forms Spec §05: a spinner
		     that flashes for one frame reads as a glitch, not as progress. -->
		<span
			class="size-3.5 animate-spin rounded-full border-2 border-current
			       border-t-transparent"
			aria-hidden="true"
		></span>
	{:else if icon}
		{@render icon()}
	{/if}
	{label}
</button>

{#if reasonId}
	<span id={reasonId} class="sr-only">{disabledReason}</span>
{/if}
