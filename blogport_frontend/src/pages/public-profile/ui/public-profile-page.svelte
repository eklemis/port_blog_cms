<script lang="ts">
	import { PublicHeader } from '$lib/widgets/public-header';

	/**
	 * `/[username]` — the front door, and the address the register and account
	 * screens promise. Screen / Public profile 82:420, Mobile 88:2062.
	 *
	 * §03's own note at 82:500 says what it is for: "A doorway, not a fourth
	 * list: every block here leads somewhere that already exists." So each strip
	 * shows a few and hands over to the list behind it, and a strip with nothing
	 * in it is not drawn at all — a door to an empty room is worse than one door
	 * fewer.
	 *
	 * **The identity actions are not here**, and that is now two answers rather
	 * than one gap. The frame draws "Senior Backend Engineer · Jakarta", View
	 * résumé, GitHub and an email address.
	 *
	 * The first three wait on the nominated public CV, which is filed: a CV
	 * carries `role` and `contact_info`, so they arrive together the day an
	 * author can choose one.
	 *
	 * **The email address is not waiting — it was ruled out on 20 September.**
	 * `PublicProfile` withholds it deliberately ("No email and no account state:
	 * this is the one endpoint that serves a user's details to somebody else"),
	 * and a scrapeable address on the page every visitor reaches would undo that
	 * on the author's behalf. Contact belongs on a résumé an author *chose* to
	 * publish, where `contact_info` is part of a document they nominated. One
	 * deliberate act, one audience.
	 */
	let {
		author,
		projects = [],
		posts = []
	}: {
		author: { username: string; fullName: string; bio?: string | null; avatarSrc?: string | null };
		projects?: {
			slug: string;
			title: string;
			description?: string | null;
			techStack: string[];
			cover: { src: string; alt: string } | null;
		}[];
		posts?: { slug: string; title: string; published: string | null }[];
	} = $props();

	const handle = $derived(encodeURIComponent(author.username));
</script>

<!-- eslint-disable svelte/no-navigation-without-resolve --
	Every href is built from a username and a slug that arrive at runtime,
	already escaped by the loader; `resolve()` would encode them twice. -->

<div class="min-h-dvh bg-arch-bg">
	<PublicHeader
		username={author.username}
		fullName={author.fullName}
		avatarSrc={author.avatarSrc ?? null}
		measure="scan"
	/>

	<div class="flex justify-center px-5 pt-[18px] pb-16 md:px-8 md:pt-[30px]">
		<div class="flex w-full max-w-[860px] flex-col gap-5 md:gap-[26px]">
			<!-- 82:435 — beside the name on a desktop, stacked above it on a phone. -->
			<div class="flex flex-col gap-3 md:flex-row md:items-center md:gap-[22px]">
				{#if author.avatarSrc}
					<img
						src={author.avatarSrc}
						alt=""
						width="88"
						height="88"
						class="size-[78px] shrink-0 rounded-full object-cover md:size-[88px]"
					/>
				{/if}
				<div class="flex min-w-0 flex-col gap-1.5 md:gap-[7px]">
					<h1 class="font-display text-[30px] font-extrabold text-arch-headline md:text-[38px]">
						{author.fullName}
					</h1>
					{#if author.bio}
						<p data-bio class="max-w-[680px] text-[15px] leading-[25px] text-arch-muted">
							{author.bio}
						</p>
					{/if}
				</div>
			</div>

			{#if projects.length}
				<div class="h-px bg-arch-line" role="presentation"></div>

				<section class="flex flex-col gap-4 md:gap-[26px]">
					<div class="flex items-center justify-between gap-3">
						<h2
							class="font-mono text-[9.5px] font-normal tracking-[1.14px] text-arch-muted uppercase"
						>
							Selected work
						</h2>
						<a href="/{handle}/projects" class="text-[11.5px] text-arch-accent-ink hover:underline">
							All projects →
						</a>
					</div>

					<ul class="grid list-none grid-cols-1 gap-4 p-0 md:grid-cols-3">
						{#each projects as project (project.slug)}
							<li
								class="flex flex-col overflow-hidden rounded-xl border border-arch-line
								       bg-arch-surface"
							>
								{#if project.cover}
									<img
										src={project.cover.src}
										alt={project.cover.alt}
										class="h-[120px] w-full object-cover"
									/>
								{/if}
								<div class="flex flex-col gap-[7px] px-4 pt-2.5 pb-4">
									<a
										href="/{handle}/projects/{encodeURIComponent(project.slug)}"
										class="text-[15px] font-semibold text-arch-headline hover:text-arch-accent-ink"
									>
										{project.title}
									</a>
									{#if project.description}
										<!-- The phone frame drops this line; there is no room for it
										     under a full-width cover. -->
										<p class="text-[12.5px] leading-[19px] text-arch-muted max-md:hidden">
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
								</div>
							</li>
						{/each}
					</ul>
				</section>
			{/if}

			{#if posts.length}
				<div class="h-px bg-arch-line" role="presentation"></div>

				<section class="flex flex-col gap-1 md:gap-[26px]">
					<div class="flex items-center justify-between gap-3">
						<h2
							class="font-mono text-[9.5px] font-normal tracking-[1.14px] text-arch-muted uppercase"
						>
							Recent writing
						</h2>
						<a href="/{handle}/blog" class="text-[11.5px] text-arch-accent-ink hover:underline">
							All posts →
						</a>
					</div>

					<ul class="flex list-none flex-col p-0">
						{#each posts as post (post.slug)}
							<li class="flex items-center justify-between gap-4 py-2 md:h-[34px] md:py-0">
								<a
									href="/{handle}/blog/{encodeURIComponent(post.slug)}"
									class="min-w-0 text-[15px] font-medium text-arch-headline
									       hover:text-arch-accent-ink"
								>
									{post.title}
								</a>
								{#if post.published}
									<span class="shrink-0 font-mono text-[11.5px] text-arch-muted">
										{post.published}
									</span>
								{/if}
							</li>
						{/each}
					</ul>
				</section>
			{/if}
		</div>
	</div>
</div>
