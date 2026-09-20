<script lang="ts">
	import { X } from '@lucide/svelte';

	/**
	 * A list of short values as removable tokens — §03's `chips` field type.
	 *
	 * "An array as removable tokens. Enter or `,` commits; Backspace on an empty
	 * input removes the last. **Never a comma-separated text field**" — because
	 * nobody should have to guess the delimiter.
	 *
	 * In the kit rather than in a feature: a project's tech stack needs it on the
	 * create form and in the editor, and slices in the same layer may not import
	 * each other.
	 *
	 * The resting input is the frame's "+ Add" pill. It is an input rather than a
	 * button that reveals one, so the first keystroke lands in the right place.
	 */
	let {
		label,
		values = [],
		help,
		onchange = () => {}
	}: {
		label: string;
		values?: string[];
		/** How it works. A chip input is not self-evident. */
		help?: string;
		onchange?: (values: string[]) => void;
	} = $props();

	let typed = $state('');

	const labelId = $props.id();

	function commit() {
		const value = typed.trim().replace(/,$/, '').trim();
		typed = '';

		if (!value || values.includes(value)) return;

		onchange([...values, value]);
	}

	function typing(value: string) {
		typed = value;

		// A comma is how people type a list; committing on it means nobody has to
		// discover that Enter is the only way.
		if (value.includes(',')) commit();
	}

	function keyed(event: KeyboardEvent) {
		if (event.key === 'Enter') {
			// Inside a form, Enter would otherwise submit it — and committing a
			// chip is what the person meant.
			event.preventDefault();
			commit();
			return;
		}

		// Only when there is nothing to delete in the field itself. Otherwise
		// Backspace is editing the word being typed.
		if (event.key === 'Backspace' && typed === '' && values.length) {
			onchange(values.slice(0, -1));
		}
	}
</script>

<div class="flex flex-col gap-1.5" role="group" aria-labelledby={labelId}>
	<span id={labelId} class="text-[11.5px] text-arch-muted">{label}</span>

	<div class="flex flex-wrap items-center gap-[7px]">
		{#each values as value (value)}
			<span
				class="flex items-center gap-1.5 rounded-full bg-arch-surface-2 px-2.5 py-[5px]
				       text-[11px] text-arch-headline"
			>
				{value}
				<button
					type="button"
					aria-label="Remove {value}"
					onclick={() => onchange(values.filter((have) => have !== value))}
					class="text-arch-muted hover:text-arch-headline"
				>
					<X size={11} aria-hidden="true" />
				</button>
			</span>
		{/each}

		<input
			type="text"
			value={typed}
			aria-label="Add to {label.toLowerCase()}"
			placeholder="+ Add"
			oninput={(event) => typing(event.currentTarget.value)}
			onkeydown={keyed}
			class="w-[76px] rounded-full border border-arch-line px-2.5 py-[5px] text-[11px]
			       text-arch-headline placeholder:text-arch-muted"
		/>
	</div>

	{#if help}
		<p class="text-[10.5px] text-arch-muted">{help}</p>
	{/if}
</div>
