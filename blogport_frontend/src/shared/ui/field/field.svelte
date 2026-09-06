<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLInputAttributes } from 'svelte/elements';

	/**
	 * Field — a labelled single-line input, in the five states the Forms &
	 * Interaction Spec §04 gives it: default, focus, filled, error, disabled.
	 *
	 * Three rules from that spec are built in rather than left to callers:
	 *
	 *   · The placeholder is an example, never the label. The label is a real
	 *     element with a for/id pair (Accessibility Spec §08).
	 *   · Error REPLACES help — the two never stack. Both reach assistive tech
	 *     through aria-describedby; an error also sets aria-invalid.
	 *   · A password is one field with a reveal toggle, never a confirm twin
	 *     (Forms Spec §01). The toggle is type="button" so revealing a password
	 *     cannot submit the form it sits in.
	 *
	 * Validation timing is the caller's: pass `onblur` and set `error` from it.
	 * Nothing here validates per keystroke — see Forms Spec §05.
	 *
	 * Design: Figma Field component set 19:23.
	 */
	type Type = 'text' | 'email' | 'password';

	let {
		label,
		value = $bindable(''),
		type = 'text',
		id,
		name,
		maxlength,
		placeholder,
		help,
		error,
		disabled = false,
		required = false,
		autocomplete,
		/** A control on the label row — "Forgot password?". Moves below the input under 768px. */
		labelAction,
		oninput,
		onblur
	}: {
		label: string;
		value?: string;
		type?: Type;
		/** Supply one when a caller needs to move focus here; otherwise it is generated. */
		id?: string;
		name?: string;
		/**
		 * Caps the field rather than reporting an overrun. Use it where the copy
		 * table names a minimum and no maximum.
		 */
		maxlength?: number;
		placeholder?: string;
		help?: string;
		error?: string;
		disabled?: boolean;
		required?: boolean;
		autocomplete?: HTMLInputAttributes['autocomplete'];
		labelAction?: Snippet;
		/** Fires per keystroke — for tracking that a field has been touched, never to validate. */
		oninput?: (event: Event) => void;
		onblur?: (event: FocusEvent) => void;
	} = $props();

	const uid = $props.id();
	const inputId = $derived(id ?? `${uid}-input`);
	const noteId = `${uid}-note`;

	let revealed = $state(false);

	const isPassword = $derived(type === 'password');
	// The reveal toggle swaps the rendered type. `value` is written by hand
	// rather than with bind:value, because Svelte forbids a dynamic `type` on a
	// two-way-bound input — and the type is exactly what the toggle changes.
	const renderedType = $derived(isPassword && revealed ? 'text' : type);

	// One slot, one message: the error wins when both exist (Forms Spec §05).
	const note = $derived(error ?? help);
</script>

<div class="flex w-full flex-col gap-1.5 {disabled ? 'opacity-55' : ''}">
	<!-- `max-md:contents` dissolves this row under 768px so its two children
	     become direct children of the column, letting the label stay put while
	     the action moves below the input — Mobile / Sign in 89:2082. -->
	<div class="flex items-center justify-between gap-3 {labelAction ? 'max-md:contents' : ''}">
		<label for={inputId} class="text-[12px] text-arch-muted">{label}</label>
		{#if labelAction}
			<div class="shrink-0 text-[11.5px] max-md:order-last max-md:mt-2 max-md:self-end">
				{@render labelAction()}
			</div>
		{/if}
	</div>

	<!--
		The padding sits on the input and the toggle rather than on this box, so
		the whole bordered rectangle is the pointer target. With it here the
		input measured 20px tall inside a 44px box, and WCAG 2.2 AA 2.5.8 wants
		24 — the box looked right and only the border was clickable.
	-->
	<div
		class="flex items-center rounded-lg border
		       {disabled ? 'bg-arch-surface-2' : 'bg-arch-surface'}
		       {error
			? 'border-[1.5px] border-st-danger'
			: 'border-arch-line-control focus-within:border-2 focus-within:border-arch-accent-ink'}"
	>
		<input
			id={inputId}
			{name}
			{maxlength}
			{placeholder}
			{disabled}
			{required}
			{autocomplete}
			type={renderedType}
			{value}
			inputmode={type === 'email' ? 'email' : undefined}
			autocapitalize={type === 'email' ? 'off' : undefined}
			spellcheck={type === 'email' ? false : undefined}
			aria-invalid={error ? 'true' : undefined}
			aria-describedby={note ? noteId : undefined}
			oninput={(event) => {
				value = event.currentTarget.value;
				oninput?.(event);
			}}
			{onblur}
			class="w-full min-w-0 bg-transparent px-3.5 py-[11px] font-mono text-[13px]
			       text-arch-headline placeholder:text-arch-muted focus:outline-none
			       disabled:cursor-not-allowed"
		/>
		{#if isPassword}
			<!-- The ring is not suppressed here: the input's focus outline is
			     replaced by the wrapper's 2px accent-ink border, which is the same
			     colour and weight. Accessibility Spec §05 allows a replacement that
			     is at least as visible; this button keeps the global ring. -->
			<button
				type="button"
				class="inline-flex shrink-0 items-center self-stretch rounded px-3.5 text-[11.5px]
				       text-arch-muted hover:text-arch-headline"
				onclick={() => (revealed = !revealed)}
			>
				{revealed ? 'Hide' : 'Show'}
			</button>
		{/if}
	</div>

	{#if note}
		<p id={noteId} class="text-[11px] {error ? 'text-st-danger' : 'text-arch-muted'}">
			{note}
		</p>
	{/if}
</div>
