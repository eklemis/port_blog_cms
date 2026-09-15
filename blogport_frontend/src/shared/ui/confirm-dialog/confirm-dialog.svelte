<script lang="ts">
	import Button from '../button/button.svelte';

	/**
	 * The third rung of destruction — Console Blueprint §06: "a dialog naming
	 * the item, requiring the title to be typed, with a danger-coloured button.
	 * Never skip a rung, and never dress a purge as an archive."
	 *
	 * Forms Spec states: idle · typing · armed · working. Every rule is about
	 * what cannot happen by accident:
	 *
	 *   · It opens with focus on Cancel (Accessibility Spec §06), never on the
	 *     button that destroys.
	 *   · Nothing confirms until the typed string matches exactly — case
	 *     included, because "nearly" is the wrong standard for something that
	 *     cannot be undone.
	 *   · Enter never confirms. Typing a name and pressing Enter is exactly the
	 *     reflex this dialog is here to interrupt.
	 *   · Escape cancels, and focus goes back to whatever opened it.
	 *
	 * A native `<dialog>` opened modally, so the focus trap and the return of
	 * focus are the platform's.
	 */
	let {
		open,
		title,
		consequence,
		match,
		confirmLabel,
		working = false,
		onconfirm,
		oncancel
	}: {
		open: boolean;
		title: string;
		/** The sentence that says what is lost. Read out as the description. */
		consequence: string;
		/** What has to be typed. The item's own name. */
		match: string;
		confirmLabel: string;
		working?: boolean;
		onconfirm: () => void;
		oncancel: () => void;
	} = $props();

	const id = $props.id();

	let dialog = $state<HTMLDialogElement>();
	let cancel = $state<HTMLButtonElement>();
	let typed = $state('');

	const armed = $derived(typed === match);

	$effect(() => {
		if (!dialog) return;

		if (open && !dialog.open) {
			typed = '';
			dialog.showModal();
			// After showModal, which would otherwise focus the first control —
			// the input, one step closer to confirming than Cancel is.
			cancel?.focus();
		} else if (!open && dialog.open) {
			dialog.close();
		}
	});
</script>

<dialog
	bind:this={dialog}
	aria-modal="true"
	aria-labelledby="{id}-title"
	aria-describedby="{id}-consequence"
	oncancel={(event) => {
		// Escape. The caller owns `open`, so the dialog does not close itself.
		event.preventDefault();
		if (!working) oncancel();
	}}
	class="m-auto w-[calc(100%-2rem)] max-w-[440px] rounded-2xl border border-arch-line
	       bg-arch-surface p-6 backdrop:bg-arch-scrim"
>
	<h2 id="{id}-title" class="font-display text-[17px] font-extrabold text-arch-headline">
		{title}
	</h2>
	<p id="{id}-consequence" class="mt-2 text-[13px] text-arch-muted">{consequence}</p>

	<div class="mt-4 flex flex-col gap-1.5">
		<label for="{id}-match" class="text-[12.5px] text-arch-headline">
			Type <strong class="font-semibold">{match}</strong> to confirm
		</label>
		<input
			id="{id}-match"
			type="text"
			autocomplete="off"
			spellcheck="false"
			bind:value={typed}
			disabled={working}
			onkeydown={(event) => {
				if (event.key === 'Enter') event.preventDefault();
			}}
			class="rounded-lg border border-arch-line-control bg-arch-surface px-3 py-2.5
			       font-mono text-[13px] text-arch-headline"
		/>
	</div>

	<div class="mt-5 flex justify-end gap-2">
		<button
			bind:this={cancel}
			type="button"
			disabled={working}
			onclick={oncancel}
			class="inline-flex min-h-11 items-center rounded-lg border border-arch-line-control
			       px-4 text-sm font-semibold text-arch-headline disabled:opacity-55"
		>
			Cancel
		</button>
		<Button
			kind="danger"
			label={confirmLabel}
			disabled={!armed}
			loading={working}
			onclick={onconfirm}
		/>
	</div>
</dialog>
