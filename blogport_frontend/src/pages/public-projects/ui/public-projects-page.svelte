<script lang="ts">
	import { PublicHeader } from '$lib/widgets/public-header';
	import { Pager } from '$lib/widgets/pager';

	/**
	 * An author's projects — Screen / Public projects 82:579 and
	 * Mobile / Public projects 88:2225.
	 *
	 * This is the **Scan** family, not the Read family: 860 at desktop and 770
	 * with 32px gutters at tablet, against the post page's 760/700. A grid of
	 * cards is scanned; an article is read, and a reading measure does not widen
	 * because the screen does.
	 *
	 * **The filter row is half here**, as on the author index. Every card now
	 * names its topics, so the *active* filter has a label and a way off — but
	 * nothing public lists the topics an author uses, so the row of chips to
	 * choose from is not built. Deriving it from the cards on the page would give
	 * a row that changes as you page and collapses to one chip once filtered.
	 *
	 * The intro paragraph the frame draws under the title is not here either: no
	 * field carries it, on this payload or the profile's.
	 */
	let {
		author,
		projects,
		total,
		page,
		perPage,
		filter = null,
		onpage = () => {}
	}: {
		author: { username: string; fullName: string; avatarSrc?: string | null };
		projects: {
			slug: string;
			title: string;
			description?: string | null;
			techStack: string[];
			topics: { id: string; title: string }[];
			cover: { src: string; alt: string } | null;
			repoUrl?: string | null;
			demoUrl?: string | null;
		}[];
		total: number;
		page: number;
		perPage: number;
		filter?: { id: string; title: string } | null;
		onpage?: (page: number) => void;
	} = $props();

	const handle = $derived(encodeURIComponent(author.username));
	const index = $derived(`/${handle}/projects`);
</script>

<!-- eslint-disable svelte/no-navigation-without-resolve --
	Every href here is built from a username and a slug that arrive at runtime,
	already escaped by the loader; `resolve()` would encode them twice. The two
	outward links are the author's own, and belong to nobody's route table. -->

<div class="min-h-dvh bg-arch-bg">
	<PublicHeader
		username={author.username}
		fullName={author.fullName}
		avatarSrc={author.avatarSrc ?? null}
		current="projects"
		measure="scan"
	/>

	<div class="flex justify-center px-5 pt-[18px] pb-16 md:px-8 md:pt-[30px]">
		<div class="flex w-full max-w-[860px] flex-col gap-[14px] md:gap-[22px]">
			<h1 class="font-display text-[28px] font-extrabold text-arch-headline md:text-[32px]">
				Projects
			</h1>

			{#if filter}
				<div class="flex items-center gap-2 overflow-x-auto">
					<span class="text-[11.5px] text-arch-muted max-md:hidden">Filter</span>
					<a
						href={index}
						aria-label="Clear the {filter.title} filter"
						class="flex shrink-0 items-center gap-1.5 rounded-full border border-arch-accent-ink
						       bg-arch-surface-2 px-[11px] py-[5px] text-[11.5px] leading-[1.2]
						       font-semibold text-arch-accent-ink"
					>
						{filter.title}
						<span aria-hidden="true" class="font-normal">×</span>
					</a>
				</div>
			{/if}

			{#if projects.length}
				<!-- Two across from md up, one on a phone. 2 × 422 + 16 is the frame's
				     860; the same grid gives 2 × 377 in the 770 a tablet leaves. -->
				<ul class="grid list-none grid-cols-1 gap-4 p-0 md:grid-cols-2">
					{#each projects as project (project.slug)}
						<li
							class="flex flex-col overflow-hidden rounded-[11px] border border-arch-line
							       bg-arch-surface md:rounded-xl"
						>
							{#if project.cover}
								<img
									src={project.cover.src}
									alt={project.cover.alt}
									class="h-[132px] w-full object-cover md:h-[150px]"
								/>
							{/if}

							<div class="flex flex-col gap-2 px-4 py-3.5 md:gap-[9px] md:px-[18px] md:py-4">
								<a
									href="{index}/{encodeURIComponent(project.slug)}"
									class="text-[16px] font-semibold text-arch-headline hover:text-arch-accent-ink
									       md:text-[17px]"
								>
									{project.title}
								</a>

								{#if project.description}
									<p
										class="text-[12.5px] leading-[19px] text-arch-muted md:text-[13px]
										       md:leading-5"
									>
										{project.description}
									</p>
								{/if}

								{#if project.techStack.length}
									<ul class="flex list-none flex-wrap gap-1.5 p-0">
										{#each project.techStack as tech (tech)}
											<li
												class="rounded-full bg-arch-surface-2 px-2.5 py-[5px] text-[11px]
												       leading-[1.2] text-arch-headline"
											>
												{tech}
											</li>
										{/each}
									</ul>
								{/if}

								{#if project.repoUrl || project.demoUrl}
									<p class="flex items-center gap-1.5 text-[11.5px] text-arch-accent-ink">
										{#if project.repoUrl}
											<a href={project.repoUrl} rel="noopener" class="hover:underline">repo</a>
										{/if}
										{#if project.repoUrl && project.demoUrl}
											<span aria-hidden="true" class="text-arch-muted">·</span>
										{/if}
										{#if project.demoUrl}
											<a href={project.demoUrl} rel="noopener" class="hover:underline">demo</a>
										{/if}
									</p>
								{/if}
							</div>
						</li>
					{/each}
				</ul>

				<Pager shown={projects.length} {total} {page} {perPage} {onpage} />
			{:else}
				<!-- No document specifies this copy; no frame draws an empty public
				     listing. Kept plain and reported rather than dressed up. -->
				<div class="flex flex-col items-start gap-2 py-10">
					<p class="text-[14px] text-arch-muted">
						{filter ? 'No projects under this topic.' : 'Nothing published yet.'}
					</p>
					{#if filter}
						<a href={index} class="text-[13px] font-semibold text-arch-accent-ink hover:underline">
							Show all projects
						</a>
					{/if}
				</div>
			{/if}
		</div>
	</div>
</div>
