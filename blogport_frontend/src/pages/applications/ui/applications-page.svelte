<script lang="ts">
	import { Plus } from '@lucide/svelte';
	import { Button, EmptyState, SkeletonRows, StatusPill } from '$lib/shared/ui';
	import type { TrackerRow } from '$lib/entities/application';
	import { CONSOLE_ROUTES } from '$lib/shared/config/routes';

	/**
	 * `/studio/applications` — the tracker.
	 *
	 * One row per application, status inline, in the order the API sent them.
	 * There is no search, filter, sort or paging: neither `GET /api/applications`
	 * nor `GET /api/jobs` takes a parameter, and a control that filters a page
	 * of rows while claiming to filter the list is worse than no control.
	 *
	 * Read-only. Inline status and next action are `PATCH /api/applications/{id}`
	 * and adding a job is `/studio/applications/new`; both are their own slices.
	 *
	 * Design: Screen / Application tracker · Screen / Applications — empty.
	 */
	let {
		rows,
		failed = false,
		loading = false
	}: {
		rows: TrackerRow[];
		failed?: boolean;
		loading?: boolean;
	} = $props();
</script>

<div class="flex flex-col gap-5">
	<div class="flex items-center justify-between gap-4">
		<h1
			class="font-display text-[24px] font-extrabold tracking-tight text-arch-headline md:text-[27px]"
		>
			Applications
		</h1>
		<Button label="Add a job" href={`${CONSOLE_ROUTES.applications}/new`}>
			{#snippet icon()}<Plus size={15} aria-hidden="true" />{/snippet}
		</Button>
	</div>

	{#if failed}
		<!-- Never blames the person; always says their work is safe. -->
		<EmptyState
			title="We couldn't load your applications."
			message="Nothing has happened to them. Try again in a moment."
		/>
	{:else if loading}
		<SkeletonRows label="Loading applications" />
	{:else if rows.length === 0}
		<!-- Why it is empty, and the one action that fixes it. -->
		<EmptyState
			title="No applications yet."
			message="Paste a job posting and this is where it will be tracked."
		>
			{#snippet action()}
				<!-- Named differently from the header's button on purpose: two links
				     with the same accessible name is a list nobody can tell apart. -->
				<Button label="Add your first job" href={`${CONSOLE_ROUTES.applications}/new`} />
			{/snippet}
		</EmptyState>
	{:else}
		<div class="overflow-x-auto rounded-xl border border-arch-line bg-arch-surface">
			<table class="w-full min-w-[560px] border-collapse text-left">
				<thead>
					<tr class="border-b border-arch-line">
						{#each ['Role', 'Status', 'Next action', 'Applied'] as heading (heading)}
							<th
								scope="col"
								class="px-[18px] py-3 font-mono text-[9px] font-normal tracking-[0.9px]
								       text-arch-muted uppercase"
							>
								{heading}
							</th>
						{/each}
					</tr>
				</thead>
				<tbody>
					{#each rows as row (row.id)}
						<tr class="border-b border-arch-line last:border-b-0">
							<td class="px-[18px] py-3">
								<p class="text-[13px] font-medium text-arch-headline">{row.role}</p>
								{#if row.company}
									<p class="text-[12px] text-arch-muted">{row.company}</p>
								{/if}
							</td>
							<td class="px-[18px] py-3">
								<StatusPill tone={row.status.tone} label={row.status.label} />
							</td>
							<!-- Dropped at tablet, per the Prototype Map's column table. -->
							<td class="hidden px-[18px] py-3 text-[12px] text-arch-muted md:table-cell">
								{row.nextAction}
							</td>
							<td class="hidden px-[18px] py-3 text-[12px] text-arch-muted md:table-cell">
								{row.applied}
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>

		<p class="text-[12px] text-arch-muted">{rows.length} applications</p>
	{/if}
</div>
