<script lang="ts">
	import { Plus } from '@lucide/svelte';
	import { Button, EmptyState, SkeletonRows, StatusPill } from '$lib/shared/ui';
	import type { TrackerRow } from '$lib/entities/application';
	import { CONSOLE_ROUTES } from '$lib/shared/config/routes';

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
	 * Two things the frame draws are not here yet, and both are reported rather
	 * than faked. The CV used column needs each snapshot's name, which the
	 * listing does not carry. And the row and its next action are links in the
	 * frame — to the tailoring, cover-letter and reflection screens, none of which
	 * exists — so they render as text until they have somewhere to go: accent ink
	 * on text that goes nowhere would promise a link that is not there.
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

	const lastPage = $derived(Math.max(1, Math.ceil(total / perPage)));
</script>

<div class="flex flex-col gap-5">
	<div class="flex items-center justify-between gap-4">
		<h1
			class="font-display text-[24px] font-extrabold tracking-tight text-arch-headline md:text-[27px]"
		>
			Applications
		</h1>
		<!-- "Add" on a phone, as the mobile frame shortens it. One is always
		     display:none, so a screen reader meets exactly one of them. -->
		<span class="max-md:hidden">
			<Button label="Add a job" href={`${CONSOLE_ROUTES.applications}/new`}>
				{#snippet icon()}<Plus size={15} aria-hidden="true" />{/snippet}
			</Button>
		</span>
		<span class="md:hidden">
			<Button label="Add" href={`${CONSOLE_ROUTES.applications}/new`} />
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
						{#each ['Role & company', 'Status', 'Applied', 'Next action'] as heading (heading)}
							<th
								scope="col"
								class="px-[18px] py-[11px] font-mono text-[9px] font-normal tracking-[0.9px]
								       text-arch-muted uppercase
								       {heading === 'Applied' ? 'max-lg:hidden' : ''}"
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
							<!-- Dropped at tablet, per the Prototype Map's column rules. -->
							<td class="px-[18px] py-[13px] text-[12px] text-arch-muted max-lg:hidden">
								{row.applied ?? '—'}
							</td>
							<td class="px-[18px] py-[13px] text-[12px] text-arch-headline">
								{row.nextAction || '—'}
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
						<p class="text-[11.5px] text-arch-headline">{row.nextAction || '—'}</p>
						<p class="font-mono text-[10.5px] text-arch-muted">
							{row.applied ? `sent ${row.applied}` : 'not sent'}
						</p>
					</div>
				</li>
			{/each}
		</ul>

		<div class="flex items-center justify-between text-[12px] text-arch-muted">
			<p>{rows.length} of {total} applications</p>
			{#if lastPage > 1}
				<div class="flex items-center gap-1">
					<Button
						kind="ghost"
						label="Previous"
						disabled={page <= 1}
						disabledReason={page <= 1 ? 'You are on the first page.' : undefined}
						onclick={() => onpage(page - 1)}
					/>
					<span class="font-mono">{page} / {lastPage}</span>
					<Button
						kind="ghost"
						label="Next"
						disabled={page >= lastPage}
						disabledReason={page >= lastPage ? 'You are on the last page.' : undefined}
						onclick={() => onpage(page + 1)}
					/>
				</div>
			{/if}
		</div>
	{/if}
</div>
