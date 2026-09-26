<script lang="ts">
	import { untrack } from 'svelte';
	import { Button, Field } from '$lib/shared/ui';
	import type { HighlightedProject } from '$lib/entities/cv';

	/**
	 * Highlighted projects — Screen / CV builder 69:2, the fourth rail card.
	 *
	 * The other three collections are typed. This one is **chosen**:
	 * `HighlightedProjectDto` carries an `id` and a `slug`, so a row's identity
	 * belongs to the project it points at. A résumé that invented them would
	 * point at nothing, which is why adding is a picker rather than a form.
	 *
	 * The one field that is the résumé's own is `short_description`, and a new
	 * row starts without one. The project's description is the project's; the
	 * line under it on a CV is written for the CV, and prefilling a paragraph
	 * into a field labelled "one-line summary" invites leaving it there.
	 *
	 * Like the others, every change reports the whole list — `highlighted_projects`
	 * is a `ReplaceOp`.
	 */
	type Project = { id: string; title: string; slug: string };

	let {
		chosen,
		projects,
		onchange
	}: {
		chosen: HighlightedProject[];
		/** Every project the author owns, for the picker to offer. */
		projects: Project[];
		onchange: (chosen: HighlightedProject[]) => void;
	} = $props();

	// A seed: the list is being edited here.
	let list = $state<HighlightedProject[]>(untrack(() => chosen.map((row) => ({ ...row }))));
	let open = $state<number | null>(null);
	let picked = $state('');

	const ids = $props.id();

	const offerable = $derived(
		projects.filter((project) => !list.some((row) => row.id === project.id))
	);

	function commit(next: HighlightedProject[]) {
		list = next;
		onchange(next);
	}

	function add() {
		const project = projects.find((candidate) => candidate.id === picked);
		if (!project) return;

		picked = '';
		open = list.length;

		commit([
			...list,
			{
				// Identity comes from the project, never from this form.
				id: project.id,
				slug: project.slug,
				title: project.title,
				short_description: ''
			}
		]);
	}
</script>

<section
	aria-label="Highlighted projects"
	class="flex flex-col gap-2.5 rounded-xl border border-arch-line bg-arch-surface p-5"
>
	<h2 class="font-mono text-[9px] font-normal tracking-[0.9px] text-arch-muted uppercase">
		Highlighted projects
	</h2>

	{#if list.length}
		<ul class="flex list-none flex-col gap-2 p-0">
			{#each list as row, index (row.id)}
				<li class="rounded-lg border border-arch-line">
					{#if open === index}
						<div class="flex flex-col gap-2.5 p-3">
							<p class="text-[12px] font-medium text-arch-headline">{row.title}</p>
							<Field
								id="{ids}-{index}-summary"
								label="One-line summary"
								value={row.short_description}
								help="What this project says about you, on this résumé."
								oninput={(event) =>
									commit(
										list.map((have, at) =>
											at === index
												? {
														...have,
														short_description: (event.currentTarget as HTMLInputElement).value
													}
												: have
										)
									)}
							/>
							<div class="flex items-center justify-between gap-2">
								<button
									type="button"
									onclick={() => (open = null)}
									class="text-[11px] text-arch-muted hover:text-arch-headline"
								>
									Done
								</button>
								<button
									type="button"
									aria-label="Remove {row.title}"
									onclick={() => {
										open = null;
										commit(list.filter((_, at) => at !== index));
									}}
									class="text-[11px] text-st-danger hover:underline"
								>
									Remove
								</button>
							</div>
						</div>
					{:else}
						<div class="flex items-center justify-between gap-2 px-3 py-2">
							<span class="min-w-0 truncate text-[11.5px] text-arch-headline">{row.title}</span>
							<button
								type="button"
								aria-label="Edit {row.title}"
								onclick={() => (open = index)}
								class="shrink-0 text-[11px] text-arch-muted hover:text-arch-headline"
							>
								Edit
							</button>
						</div>
					{/if}
				</li>
			{/each}
		</ul>
	{/if}

	{#if !projects.length}
		<p class="text-[11.5px] text-arch-muted">No projects yet — add one under Projects first.</p>
	{:else if !offerable.length}
		<p class="text-[11.5px] text-arch-muted">Every project is already on this résumé.</p>
	{:else}
		<div class="flex flex-col gap-2">
			<label class="flex flex-col gap-[5px]">
				<span class="text-[11.5px] text-arch-muted">Project to highlight</span>
				<select
					bind:value={picked}
					class="h-[34px] rounded-[7px] border border-arch-line-control bg-arch-surface px-3
					       text-[12.5px] text-arch-headline"
				>
					<option value="">Choose a project…</option>
					{#each offerable as project (project.id)}
						<option value={project.id}>{project.title}</option>
					{/each}
				</select>
			</label>
			<div>
				<Button size="compact" label="Add project" disabled={!picked} onclick={add} />
			</div>
		</div>
	{/if}
</section>
