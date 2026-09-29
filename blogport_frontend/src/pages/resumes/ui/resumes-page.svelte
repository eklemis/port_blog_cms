<script lang="ts">
	import { untrack } from 'svelte';
	import { Plus, Search } from '@lucide/svelte';
	import { Button, EmptyState, InlineAlert } from '$lib/shared/ui';
	import { CONSOLE_ROUTES } from '$lib/shared/config/routes';
	import { sectionsLine } from '$lib/entities/cv';
	import { createCv } from '$lib/features/create-cv';
	import type { HandlingClass } from '$lib/shared/lib/error-class';
	import { Pager } from '$lib/widgets/pager';

	/**
	 * `/studio/resumes` — Screen / Résumés 241:5528.
	 *
	 * Three columns: Résumé, Summary, Sections. Search and a sort, and no topic
	 * filter — §02: "no topic filter, because a CV carries no topics."
	 *
	 * **Where every other list has UPDATED, this one has SECTIONS**, and that is
	 * a gap being shown rather than a choice being made: `CvResponse` carries no
	 * `updated_at` while `CVSort` offers `updated_newest`, so the list can be
	 * sorted by a date it cannot display. The sort is offered because it works;
	 * the column is absent because the field is. Filed with the backend, with
	 * two others on the same document.
	 */
	type Row = {
		id: string;
		role: string;
		bio: string;
		experiences: unknown[];
		highlighted_projects: unknown[];
	};

	let {
		resumes,
		total,
		everything = null,
		page,
		perPage,
		filtered,
		failed = false,
		search = '',
		sort = null,
		displayName,
		onquery,
		oncreated,
		fetchFn = undefined
	}: {
		resumes: Row[];
		total: number;
		/** Every résumé, unfiltered — for the sentence a filtered-empty list needs. */
		everything?: number | null;
		page: number;
		perPage: number;
		filtered: boolean;
		failed?: boolean;
		search?: string;
		sort?: string | null;
		/** The name a new document is born with. The only field known before the builder. */
		displayName: string;
		/** Every change writes to the URL; the caller decides how. */
		onquery: (changes: Record<string, string | null>) => void;
		/** A new résumé exists at this id, and the builder is where it gets written. */
		oncreated: (id: string) => void;
		fetchFn?: typeof globalThis.fetch;
	} = $props();

	// A seed, not a binding: the field is the person's from the moment it
	// renders, and a loader that re-ran would otherwise overwrite their typing.
	let typed = $state(untrack(() => search));
	let timer: ReturnType<typeof setTimeout> | undefined;

	let creating = $state(false);
	let failure = $state<string | undefined>();
	let failureKind = $state<HandlingClass>('notOurs');

	/** 300ms, the posts list's figure. The two should not feel different. */
	function searched(value: string) {
		typed = value;
		clearTimeout(timer);
		timer = setTimeout(() => onquery({ search: value || null, page: null }), 300);
	}

	async function add() {
		if (creating) return;

		creating = true;
		failure = undefined;

		const result = await createCv(displayName, fetchFn);

		creating = false;

		if (!result.ok) {
			failure = result.message;
			failureKind = result.kind;
			return;
		}

		oncreated(result.id);
	}

	/**
	 * A row is named by its role, and a row with no role yet is not a blank cell.
	 *
	 * The document is born empty — J6 names it in the builder — so this is the
	 * one moment that state is visible, and a row nobody can click is the one
	 * thing it must not become.
	 */
	const title = (row: Row) => row.role.trim() || 'Untitled résumé';
</script>

<!-- eslint-disable svelte/no-navigation-without-resolve --
	Row hrefs are built from ids that arrive at runtime. -->

<div class="flex flex-col gap-4">
	<div class="flex items-center justify-between gap-4">
		<h1 class="font-display text-[19px] font-bold text-arch-headline">Résumés</h1>
		<Button label="New résumé" loading={creating} onclick={add}>
			{#snippet icon()}<Plus size={15} aria-hidden="true" />{/snippet}
		</Button>
	</div>

	<!-- 241:5528 draws this above the controls. J6's reason for the plural, in
	     the place someone first meets it. -->
	<p class="rounded-lg bg-arch-surface px-4 py-3 text-[11.5px] text-arch-muted">
		Several résumés is the point — one tailored per audience. Name each by the role it is for.
	</p>

	<InlineAlert message={failure} kind={failureKind} />

	<div class="flex flex-wrap items-center gap-2.5">
		<label class="relative flex items-center">
			<Search size={14} class="absolute left-3 text-arch-muted" aria-hidden="true" />
			<input
				type="search"
				value={typed}
				aria-label="Search résumés"
				placeholder="Search résumés…"
				oninput={(event) => searched(event.currentTarget.value)}
				class="h-[34px] w-[240px] rounded-[7px] border border-arch-line-control bg-arch-surface
				       pr-3 pl-9 text-[12.5px] text-arch-headline placeholder:text-arch-muted"
			/>
		</label>

		<select
			aria-label="Sort"
			value={sort ?? 'updated_newest'}
			onchange={(event) => onquery({ sort: event.currentTarget.value, page: null })}
			class="h-[34px] rounded-[7px] border-0 bg-transparent px-1 text-[12.5px] text-arch-muted"
		>
			<option value="updated_newest">Recently updated</option>
			<option value="newest">Newest</option>
			<option value="oldest">Oldest</option>
		</select>
	</div>

	{#if failed}
		<InlineAlert
			message="That list didn’t load. Nothing you’ve written is affected."
			kind="notOurs"
		/>
	{:else if resumes.length}
		<!-- One table, reshaped rather than duplicated. Mobile / Résumés 243:5720
		     draws cards — role, a counts pill, then the summary — and a second
		     hidden copy of every row would be a second copy of every link. Below
		     `md` the table elements become blocks, which drops the grid
		     semantics along with the column headers; at that width the headers
		     are not drawn either, and each cell carries its own meaning. -->
		<div class="md:rounded-xl md:border md:border-arch-line md:bg-arch-surface">
			<table class="block w-full border-collapse text-left md:table">
				<thead class="hidden md:table-header-group">
					<tr class="border-b border-arch-line">
						{#each ['Résumé', 'Summary', 'Sections'] as column (column)}
							<th
								scope="col"
								class="px-5 py-3 font-mono text-[9px] font-normal tracking-[0.9px]
								       text-arch-muted uppercase"
							>
								{column}
							</th>
						{/each}
					</tr>
				</thead>
				<tbody class="flex flex-col gap-2.5 md:table-row-group">
					{#each resumes as resume (resume.id)}
						<tr
							class="flex flex-col gap-2 rounded-xl border border-arch-line bg-arch-surface p-4
							       md:table-row md:rounded-none md:border-0 md:border-b md:p-0 md:last:border-0"
						>
							<td class="md:px-5 md:py-3.5">
								<a
									href="{CONSOLE_ROUTES.resumes}/{encodeURIComponent(resume.id)}"
									class="text-[15px] font-bold text-arch-headline hover:text-arch-accent-ink
									       md:text-[13px] md:font-medium"
								>
									{title(resume)}
								</a>
							</td>
							<!-- The bio, clamped rather than cut in the table: a summary
							     ending in an ellipsis mid-word is a sentence nobody wrote.
							     On a card it is short enough to read whole. -->
							<td
								class="order-last text-[12.5px] leading-[19px] text-arch-headline
								       md:order-none md:max-w-[420px] md:px-5 md:py-3.5 md:text-[12px] md:leading-[18px]"
							>
								<span class="md:line-clamp-2">{resume.bio}</span>
							</td>
							<td class="md:px-5 md:py-3.5 md:whitespace-nowrap">
								<!-- A pill on the card, plain text in the column: on a card
								     it is the only thing between two runs of prose. -->
								<span
									class="inline-block rounded-full bg-arch-surface-2 px-2.5 py-[5px] text-[11px]
									       leading-[1.2] text-arch-muted
									       md:rounded-none md:bg-transparent md:p-0 md:text-[11.5px]"
								>
									{sectionsLine(resume)}
								</span>
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>

		<Pager
			shown={resumes.length}
			{total}
			{page}
			{perPage}
			noun="résumés"
			onpage={(next) => onquery({ page: String(next) })}
		/>
	{:else if filtered}
		<EmptyState
			title="No résumés match that."
			message={everything
				? `You have ${everything} ${everything === 1 ? 'résumé' : 'résumés'}; none of them match this search.`
				: 'Nothing here matches this search.'}
		>
			{#snippet action()}
				<Button
					kind="secondary"
					label="Clear search"
					onclick={() => {
						typed = '';
						onquery({ search: null, page: null });
					}}
				/>
			{/snippet}
		</EmptyState>
	{:else}
		<!-- No second "New résumé" here: the header's is on screen, and two
		     buttons with one name are two things to tell apart by position. -->
		<EmptyState
			title="No résumés yet."
			message="One document per audience. This one starts empty and is named as you write it."
		/>
	{/if}
</div>
