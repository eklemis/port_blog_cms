<script lang="ts">
	import { untrack } from 'svelte';
	import { X } from '@lucide/svelte';
	import { Field } from '$lib/shared/ui';
	import { experienceSummary, isCurrent, years, type Experience } from '$lib/entities/cv';

	/**
	 * The experience collection — Screen / CV builder 69:2.
	 *
	 * The frame states the rule under the list and it is the whole shape of this
	 * component: "Rows collapse to a one-line summary. **One opens at a time** —
	 * ten expanded rows is a wall, not a form."
	 *
	 * Every change reports the whole list back rather than one field, because
	 * `experiences` is a `ReplaceOp` — the caller has to send all of it or none.
	 * Rows are spread rather than rebuilt, so `achievements` and `description`,
	 * which this form does not draw, survive a save that never touched them.
	 */
	let {
		roles,
		onchange
	}: {
		roles: Experience[];
		/** The whole list, because the whole list is what gets saved. */
		onchange: (roles: Experience[]) => void;
	} = $props();

	// A seed, not a binding. The list is being edited here and a loader that
	// re-ran would otherwise close the row somebody was typing in.
	let list = $state<Experience[]>(untrack(() => roles.map((role) => ({ ...role }))));
	let open = $state<number | null>(null);

	const ids = $props.id();

	/** Every change reports the whole list: `experiences` is replaced wholesale. */
	function commit(next: Experience[]) {
		list = next;
		onchange(next);
	}

	function edit(index: number, patch: Partial<Experience>) {
		// Spread rather than rebuilt, so `achievements` and `description` — on
		// the DTO, not on this form — survive an edit that never touched them.
		commit(list.map((role, at) => (at === index ? { ...role, ...patch } : role)));
	}

	function addRole() {
		const blank: Experience = {
			company: '',
			position: '',
			location: '',
			start_date: '',
			end_date: '',
			tasks: [],
			achievements: [],
			description: ''
		};

		// Opened straight away: nobody adds a role in order to leave it shut.
		open = list.length;
		commit([...list, blank]);
	}

	function removeRole(index: number) {
		open = null;
		commit(list.filter((_, at) => at !== index));
	}
</script>

<section
	aria-label="Experience"
	class="flex flex-col gap-3.5 rounded-xl border border-arch-line bg-arch-surface p-5"
>
	<div class="flex items-center justify-between gap-3">
		<h2 class="font-mono text-[9px] font-normal tracking-[0.9px] text-arch-muted uppercase">
			Experience
		</h2>
		<button
			type="button"
			onclick={addRole}
			class="text-[11.5px] text-arch-accent-ink hover:underline"
		>
			+ Add role
		</button>
	</div>

	<ul class="flex list-none flex-col gap-2.5 p-0">
		{#each list as role, index (index)}
			{@const summary = experienceSummary(role)}
			<li class="rounded-lg border border-arch-line">
				{#if open === index}
					<div class="flex flex-col gap-3 p-3.5">
						<div class="flex items-center justify-between gap-3">
							<p class="text-[13px] font-semibold text-arch-headline">{summary}</p>
							<!-- Named for what it removes: a task's remove is on the same
							     screen, and "Remove" alone names both. -->
							<button
								type="button"
								aria-label="Remove {summary}"
								onclick={() => removeRole(index)}
								class="shrink-0 text-[12px] text-st-danger hover:underline"
							>
								Remove
							</button>
						</div>

						<div class="grid grid-cols-1 gap-2.5 md:grid-cols-3">
							<Field
								id="{ids}-{index}-company"
								label="Company"
								value={role.company}
								oninput={(event) =>
									edit(index, { company: (event.currentTarget as HTMLInputElement).value })}
							/>
							<Field
								id="{ids}-{index}-position"
								label="Position"
								value={role.position}
								oninput={(event) =>
									edit(index, { position: (event.currentTarget as HTMLInputElement).value })}
							/>
							<Field
								id="{ids}-{index}-location"
								label="Location"
								value={role.location}
								oninput={(event) =>
									edit(index, { location: (event.currentTarget as HTMLInputElement).value })}
							/>
						</div>

						<div class="flex flex-wrap items-end gap-2.5">
							<div class="w-[150px]">
								<Field
									id="{ids}-{index}-start"
									label="Start"
									value={role.start_date}
									oninput={(event) =>
										edit(index, {
											start_date: (event.currentTarget as HTMLInputElement).value
										})}
								/>
							</div>
							<div class="w-[150px]">
								<Field
									id="{ids}-{index}-end"
									label="End"
									value={role.end_date ?? ''}
									disabled={isCurrent(role)}
									oninput={(event) =>
										edit(index, { end_date: (event.currentTarget as HTMLInputElement).value })}
								/>
							</div>

							<label class="flex items-center gap-2 pb-2.5 text-[12px] text-arch-headline">
								<input
									type="checkbox"
									checked={isCurrent(role)}
									onchange={(event) =>
										edit(index, {
											// The absence of an end date is what "now" means —
											// `end_date` is "Absent for a current position".
											end_date: event.currentTarget.checked ? '' : role.end_date || ''
										})}
									class="size-3.5 accent-arch-accent"
								/>
								I work here now
							</label>
						</div>

						<div class="flex flex-col gap-1.5">
							<span class="text-[11.5px] text-arch-muted">Tasks</span>
							{#each role.tasks as task, taskIndex (taskIndex)}
								<div class="flex items-center gap-2">
									<input
										type="text"
										value={task}
										aria-label="Task {taskIndex + 1}"
										oninput={(event) =>
											edit(index, {
												tasks: role.tasks.map((have, at) =>
													at === taskIndex ? event.currentTarget.value : have
												)
											})}
										class="flex-1 rounded-lg border border-arch-line-control bg-arch-surface px-3
										       py-2 text-[12.5px] text-arch-headline"
									/>
									<button
										type="button"
										aria-label="Remove task {taskIndex + 1}"
										onclick={() =>
											edit(index, { tasks: role.tasks.filter((_, at) => at !== taskIndex) })}
										class="text-arch-muted hover:text-arch-headline"
									>
										<X size={13} aria-hidden="true" />
									</button>
								</div>
							{/each}
							<button
								type="button"
								onclick={() => edit(index, { tasks: [...role.tasks, ''] })}
								class="self-start text-[11.5px] text-arch-accent-ink hover:underline"
							>
								+ Add a task
							</button>
						</div>
					</div>
				{:else}
					<div class="flex items-center justify-between gap-3 px-3.5 py-2.5">
						<div class="flex min-w-0 flex-col">
							<span class="truncate text-[12.5px] text-arch-headline">{summary}</span>
							{#if years(role)}
								<span class="text-[11.5px] text-arch-muted">{years(role)}</span>
							{/if}
						</div>
						<button
							type="button"
							aria-label="Edit {summary}"
							onclick={() => (open = index)}
							class="shrink-0 text-[12px] text-arch-muted hover:text-arch-headline"
						>
							Edit
						</button>
					</div>
				{/if}
			</li>
		{/each}
	</ul>

	<p class="text-[10.5px] text-arch-muted">
		Rows collapse to a one-line summary. One opens at a time — ten expanded rows is a wall, not a
		form.
	</p>
</section>
