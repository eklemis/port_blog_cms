<script lang="ts">
	import { Button } from '$lib/shared/ui';

	/**
	 * Publish, or schedule — J4 step five. One control with two outcomes.
	 *
	 * The thing it exists to say out loud: a future date goes live with no
	 * further call, and nothing happens on screen when it does. Someone who
	 * schedules a post and then closes the tab has to already know that.
	 *
	 * Publishing now sends no date at all and lets the caller stamp it, so the
	 * timestamp is the one the request carried rather than one this component
	 * decided a moment earlier.
	 */
	let {
		publishedAt = null,
		busy = false,
		onpublish,
		onunpublish,
		/** Injected by the spec, so "in the future" is testable. */
		now = new Date()
	}: {
		publishedAt?: string | null;
		busy?: boolean;
		/** `null` means now. */
		onpublish: (at: string | null) => void;
		onunpublish: () => void;
		now?: Date;
	} = $props();

	let scheduling = $state(false);
	let when = $state('');
	let problem = $state<string | undefined>();

	const live = $derived(Boolean(publishedAt));

	function schedule() {
		const at = new Date(when);

		if (!when || Number.isNaN(at.getTime()) || at.getTime() <= now.getTime()) {
			// They pressed "Schedule", not "Publish". Quietly publishing now
			// would be the control doing the other of its two things.
			problem = 'Pick a time in the future.';
			return;
		}

		problem = undefined;
		onpublish(at.toISOString());
	}
</script>

<!--
	The buttons at the right-hand end of the editor's top bar — Screen / Post
	editor 32:934. Scheduling opens a small panel under them rather than a
	second screen, because the sentence it has to say is short and the decision
	belongs next to the button that made it.
-->
<div class="relative flex items-center gap-2">
	{#if live}
		<Button kind="secondary" label="Unpublish" disabled={busy} onclick={onunpublish} />
	{:else}
		<Button
			kind="secondary"
			label="Schedule"
			disabled={busy}
			onclick={() => (scheduling = !scheduling)}
		/>
		<Button label="Publish" disabled={busy} onclick={() => onpublish(null)} />

		{#if scheduling}
			<div
				class="absolute top-full right-0 z-10 mt-2 flex w-[300px] flex-col gap-3 rounded-xl border
				       border-arch-line bg-arch-surface p-4 shadow-lg"
			>
				<div class="flex flex-col gap-1.5">
					<label for="publish-at" class="text-[12px] text-arch-muted">Goes live</label>
					<input
						id="publish-at"
						type="datetime-local"
						bind:value={when}
						aria-invalid={problem ? 'true' : undefined}
						aria-describedby={problem ? 'publish-at-error' : 'publish-at-help'}
						class="rounded-lg border bg-arch-surface px-3 py-2.5 text-[13px] text-arch-headline
						       {problem ? 'border-st-danger' : 'border-arch-line-control'}"
					/>
					{#if problem}
						<p id="publish-at-error" class="text-[11px] text-st-danger">{problem}</p>
					{:else}
						<!-- Said before the fact, because nothing will say it afterwards. -->
						<p id="publish-at-help" class="text-[11px] text-arch-muted">
							It goes live on its own at that time. Nothing else has to happen, and nothing will
							show here when it does.
						</p>
					{/if}
				</div>
				<div class="flex justify-end gap-2">
					<Button
						kind="ghost"
						size="compact"
						label="Cancel"
						disabled={busy}
						onclick={() => (scheduling = false)}
					/>
					<Button size="compact" label="Schedule post" disabled={busy} onclick={schedule} />
				</div>
			</div>
		{/if}
	{/if}
</div>
