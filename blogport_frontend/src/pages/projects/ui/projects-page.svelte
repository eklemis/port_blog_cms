<script lang="ts">
	import { untrack } from 'svelte';
	import { Plus, Search } from '@lucide/svelte';
	import { Button, EmptyState, InlineAlert } from '$lib/shared/ui';
	import { CONSOLE_ROUTES } from '$lib/shared/config/routes';
	import { relativeDate } from '$lib/shared/lib/relative-time';
	import { linksOf, type ProjectCard } from '$lib/entities/project';
	import { Pager } from '$lib/widgets/pager';

	/**
	 * `/studio/projects` — Screen / Projects list 70:2.
	 *
	 * Four columns: Project, Stack, Links, Updated. No status column, because a
	 * project has no status — §02: "Projects have no `published_at`. Creating one
	 * publishes it." The frame says that out loud above the table rather than
	 * leaving someone to find out by adding one.
	 *
	 * Searching and filtering go back to the server. A page of ten filtered in
	 * the browser hides rows on this page and misses every match on the others.
	 */
	type Topic = { id: string; title: string };

	let {
		projects,
		topics,
		total,
		everything = null,
		page,
		perPage,
		filtered,
		failed = false,
		search = '',
		topic = null,
		sort = null,
		onquery
	}: {
		projects: ProjectCard[];
		/** The filter's options. Empty means the request for them failed. */
		topics: Topic[];
		total: number;
		/**
		 * Every project, unfiltered — for the filtered-empty sentence, where
		 * `total` is the zero that brought someone there.
		 */
		everything?: number | null;
		page: number;
		perPage: number;
		filtered: boolean;
		failed?: boolean;
		search?: string;
		topic?: string | null;
		sort?: string | null;
		/** Every change writes to the URL; the caller decides how. */
		onquery: (changes: Record<string, string | null>) => void;
	} = $props();

	// A seed, not a binding: the field is the person's from the moment it
	// renders, and a loader that re-ran would otherwise overwrite their typing.
	let typed = $state(untrack(() => search));
	let timer: ReturnType<typeof setTimeout> | undefined;

	/**
	 * A search is a request, so it waits for a pause in typing. 300ms is the
	 * posts list's figure and the two should not feel different.
	 */
	function searched(value: string) {
		typed = value;
		clearTimeout(timer);
		timer = setTimeout(() => onquery({ search: value || null, page: null }), 300);
	}

	const NEW = `${CONSOLE_ROUTES.projects}/new`;
</script>

<!-- eslint-disable svelte/no-navigation-without-resolve --
	Row hrefs are built from ids that arrive at runtime. -->

<div class="flex flex-col gap-4">
	<div class="flex items-center justify-between gap-4">
		<h1 class="font-display text-[19px] font-bold text-arch-headline">Projects</h1>
		<Button label="Add project" href={NEW}>
			{#snippet icon()}<Plus size={15} aria-hidden="true" />{/snippet}
		</Button>
	</div>

	<!-- 70:2 draws this above the table. §02 asks for the fact to be stated
	     rather than discovered by adding one. -->
	<p class="rounded-lg bg-arch-surface px-4 py-3 text-[11.5px] text-arch-muted">
		Projects have no draft state — adding one publishes it to your public page immediately.
	</p>

	<div class="flex flex-wrap items-center gap-2.5">
		<label class="relative flex items-center">
			<Search size={14} class="absolute left-3 text-arch-muted" aria-hidden="true" />
			<input
				type="search"
				value={typed}
				aria-label="Search projects"
				placeholder="Search projects…"
				oninput={(event) => searched(event.currentTarget.value)}
				class="h-[34px] w-[240px] rounded-[7px] border border-arch-line-control bg-arch-surface
				       pr-3 pl-9 text-[12.5px] text-arch-headline placeholder:text-arch-muted"
			/>
		</label>

		<select
			aria-label="Topic"
			value={topic ?? ''}
			onchange={(event) => onquery({ topic_id: event.currentTarget.value || null, page: null })}
			class="h-[34px] rounded-[7px] border border-arch-line-control bg-arch-surface px-3
			       text-[12.5px] text-arch-headline"
		>
			<option value="">All topics</option>
			{#each topics as option (option.id)}
				<option value={option.id}>{option.title}</option>
			{/each}
		</select>

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
	{:else if projects.length}
		<div class="overflow-x-auto rounded-xl border border-arch-line bg-arch-surface">
			<table class="w-full border-collapse text-left">
				<thead>
					<tr class="border-b border-arch-line">
						{#each ['Project', 'Stack', 'Links', 'Updated'] as column (column)}
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
				<tbody>
					{#each projects as project (project.id)}
						<tr class="border-b border-arch-line last:border-0">
							<td class="px-5 py-3.5">
								<a
									href="{CONSOLE_ROUTES.projects}/{encodeURIComponent(project.id)}"
									class="text-[13px] font-medium text-arch-headline hover:text-arch-accent-ink"
								>
									{project.title}
								</a>
							</td>
							<td class="px-5 py-3.5">
								<ul class="flex list-none flex-wrap gap-1.5 p-0">
									{#each project.tech_stack as tech (tech)}
										<li
											class="rounded-full bg-arch-surface-2 px-2.5 py-[5px] text-[11px]
											       leading-[1.2] text-arch-headline"
										>
											{tech}
										</li>
									{/each}
								</ul>
							</td>
							<td class="px-5 py-3.5 text-[11.5px]">
								{#each linksOf(project) as link, index (link.label)}
									{#if index > 0}<span class="px-1 text-arch-muted">·</span>{/if}
									<a
										href={link.href}
										rel="noreferrer"
										target="_blank"
										class="text-arch-accent-ink hover:underline"
									>
										{link.label}
									</a>
								{/each}
							</td>
							<td class="px-5 py-3.5 font-mono text-[11.5px] text-arch-muted">
								{relativeDate(project.updated_at)}
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>

		<Pager
			shown={projects.length}
			{total}
			{page}
			{perPage}
			noun="projects"
			onpage={(next) => onquery({ page: String(next) })}
		/>
	{:else if filtered}
		<EmptyState
			title="No projects match that."
			message={everything
				? `You have ${everything} ${everything === 1 ? 'project' : 'projects'}; none of them match these filters.`
				: 'Nothing here matches these filters.'}
		>
			{#snippet action()}
				<Button
					kind="secondary"
					label="Clear filters"
					onclick={() => {
						typed = '';
						onquery({ search: null, topic_id: null, page: null });
					}}
				/>
			{/snippet}
		</EmptyState>
	{:else}
		<EmptyState
			title="No projects yet."
			message="A project is a piece of work with an address, a stack and somewhere to see it."
		>
			{#snippet action()}
				<Button label="Add project" href={NEW}>
					{#snippet icon()}<Plus size={15} aria-hidden="true" />{/snippet}
				</Button>
			{/snippet}
		</EmptyState>
	{/if}
</div>
