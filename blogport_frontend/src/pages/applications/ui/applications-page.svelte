<script lang="ts">
	import { Plus } from '@lucide/svelte';
	import { Button, EmptyState, SkeletonRows, StatusPill } from '$lib/shared/ui';
	import type { TrackerRow } from '$lib/entities/application';
	import { CONSOLE_ROUTES } from '$lib/shared/config/routes';
	import { Pager } from '$lib/widgets/pager';

	/**
	 * The Prototype Map's column rules for 834: "Applications loses CV used and
	 * Applied". Identity, status and the action stay — three load-bearing
	 * columns, and the middle goes.
	 */
	const DROPPED_AT_TABLET = ['CV used', 'Applied'];

	/**
	 * `/studio/applications` — the tracker.
	 *
	 * One row per application, status inline, in the order the API sent them.
	 * Paged, since both listings page now — but still no search, filter or sort:
	 * neither endpoint takes one, and a control that filters a page of rows
	 * while claiming to filter the list is worse than no control.
	 *
	 * Read-only. Inline status and next action are `PATCH /api/applications/{id}`
	 * and adding a job is `/studio/applications/new`; both are their own slices.
	 *
	 * Design: Screen / Application tracker 12:118 · Mobile / Application tracker
	 * 73:120 · Screen / Applications — empty 84:701. A table from 768px; below it
	 * "five columns cannot survive 390px", and each row becomes a card.
	 *
	 * The CV used column the frame draws is not here: the listing carries a
	 * snapshot id and no name, so filling it would be one request per row.
	 *
	 * The next-action cell is two things — a step derived from the status, or
	 * what the person wrote — and neither is a link. A row is not a link either;
	 * the designer has ruled that clicking one goes nowhere until the detail
	 * screen exists, and the derived actions stay plain text until theirs do.
	 */
	let {
		rows,
		total,
		page,
		perPage,
		failed = false,
		loading = false,
		onpage = () => {}
	}: {
		rows: TrackerRow[];
		total: number;
		page: number;
		perPage: number;
		failed?: boolean;
		loading?: boolean;
		/** Paging writes to the URL; the caller decides how. */
		onpage?: (page: number) => void;
	} = $props();
</script>

<div class="flex flex-col gap-5">
	<div class="flex items-center justify-between gap-4">
		<h1
			class="font-display text-[24px] font-extrabold tracking-tight text-arch-headline max-md:sr-only md:text-[27px]"
		>
			Applications
		</h1>
		<!-- On a phone the shell's header carries "Add", as Mobile / Application
		     tracker draws it. -->
		<span class="max-md:hidden">
			<Button label="Add a job" href={`${CONSOLE_ROUTES.applications}/new`}>
				{#snippet icon()}<Plus size={15} aria-hidden="true" />{/snippet}
			</Button>
		</span>
	</div>

	{#if failed}
		<!-- CollectionState / error: never blames the person, says the work is safe. -->
		<EmptyState
			tone="danger"
			title="Couldn’t load your applications"
			message="Something went wrong on our side. Your applications are safe."
		/>
	{:else if loading}
		<SkeletonRows label="Loading applications" />
	{:else if rows.length === 0}
		<!-- Screen / Applications — empty 84:701: the one action that fixes it. -->
		<EmptyState
			title="No applications yet"
			message="Paste a job posting and the tracker starts from there."
		>
			{#snippet action()}
				<!-- Named differently from the header's button on purpose: two links
				     with the same accessible name is a list nobody can tell apart. -->
				<Button
					size="compact"
					label="Add your first job"
					href={`${CONSOLE_ROUTES.applications}/new`}
				/>
			{/snippet}
		</EmptyState>
	{:else}
		<!-- From 768px: the frame's table. -->
		<div class="overflow-x-auto rounded-xl border border-arch-line bg-arch-surface max-md:hidden">
			<table class="w-full border-collapse text-left">
				<thead>
					<tr class="border-b border-arch-line">
						{#each ['Role & company', 'Status', 'CV used', 'Applied', 'Next action'] as heading (heading)}
							<th
								scope="col"
								class="px-[18px] py-[11px] font-mono text-[9px] font-normal tracking-[0.9px]
								       text-arch-muted uppercase
								       {DROPPED_AT_TABLET.includes(heading) ? 'max-lg:hidden' : ''}"
							>
								{heading}
							</th>
						{/each}
					</tr>
				</thead>
				<tbody>
					{#each rows as row (row.id)}
						<tr class="border-b border-arch-line last:border-b-0">
							<td class="px-[18px] py-[13px] text-[13px] font-medium text-arch-headline">
								{row.company ? `${row.role} · ${row.company}` : row.role}
							</td>
							<td class="px-[18px] py-[13px]">
								<StatusPill tone={row.status.tone} label={row.status.label} />
							</td>
							<!-- Which CV went, and when. Dropped at tablet with Applied. -->
							<td class="px-[18px] py-[13px] font-mono text-[11px] text-arch-muted max-lg:hidden">
								{row.cvUsed}
							</td>
							<td class="px-[18px] py-[13px] font-mono text-[11px] text-arch-muted max-lg:hidden">
								{row.applied ?? '—'}
							</td>
							<!-- Headline colour, never accent ink: the derived actions lead to
							     screens that do not exist yet, and a person's own note was never
							     a link at all. -->
							<td class="px-[18px] py-[13px] text-[12px] text-arch-headline">
								{row.nextAction.text ?? '—'}
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>

		<!-- Below 768px: a card per row — role and status on one line, company
		     under it, next action and sent date in the footer. -->
		<ul class="flex flex-col gap-3 md:hidden">
			{#each rows as row (row.id)}
				<li
					class="flex flex-col gap-[9px] rounded-[11px] border border-arch-line bg-arch-surface
					       px-[15px] py-3.5"
				>
					<div class="flex items-center justify-between gap-3">
						<p class="text-[14px] font-semibold text-arch-headline">{row.role}</p>
						<StatusPill tone={row.status.tone} label={row.status.label} />
					</div>
					{#if row.company}
						<p class="text-[12.5px] text-arch-muted">{row.company}</p>
					{/if}
					<div class="h-px bg-arch-line" role="presentation"></div>
					<div class="flex items-center justify-between gap-3">
						<p class="text-[11.5px] text-arch-headline">{row.nextAction.text ?? '—'}</p>
						<p class="font-mono text-[10.5px] text-arch-muted">
							{row.applied ? `sent ${row.applied}` : 'not sent'}
						</p>
					</div>
				</li>
			{/each}
		</ul>

		<Pager shown={rows.length} {total} {page} {perPage} noun="applications" {onpage} />
	{/if}
</div>
