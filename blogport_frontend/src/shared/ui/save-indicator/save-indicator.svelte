<script lang="ts">
	import type { SaveState } from '$lib/shared/lib/save-state';

	/**
	 * The one place that reports save state — J4 is explicit that it is the only
	 * one. Four states in a fixed slot, so nothing below moves as they change.
	 *
	 * Filed under widgets in §08's component table and built here instead: it
	 * holds no state and knows nothing about saving, and a feature may not
	 * import a widget under the layer rule. Presentational, like StatusPill.
	 *
	 * Polite, and only the failure is coloured. Amber on "Saving…" would make an
	 * ordinary keystroke look like a problem, and a region that interrupts on
	 * every debounce tick is the over-announcement §15 warns about by name.
	 */
	let {
		state,
		savedAt = null
	}: {
		state: SaveState;
		/** Epoch milliseconds. See the autosave loop for why not a `Date`. */
		savedAt?: number | null;
	} = $props();

	const time = $derived(
		savedAt === null || savedAt === undefined
			? null
			: new Intl.DateTimeFormat(undefined, { hour: '2-digit', minute: '2-digit' }).format(savedAt)
	);

	const label = $derived(
		{
			// No time before the first save: "Saved —" is worse than saying less.
			saved: time ? `Saved ${time}` : 'Saved',
			unsaved: 'Unsaved changes',
			saving: 'Saving…',
			retrying: "Couldn't save — retrying"
		}[state]
	);
</script>

<!--
	Named, because an editor has a second polite region on it for failures and a
	screen reader needs to tell the two apart. The name does not replace what is
	announced; the changed text is still what gets read.
-->
<p
	role="status"
	aria-label="Save state"
	class="text-[12px] {state === 'retrying' ? 'text-st-danger' : 'text-arch-muted'}"
>
	{label}
</p>
