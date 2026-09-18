<script lang="ts">
	import { PublicHeader } from '$lib/widgets/public-header';
	import { Button } from '$lib/shared/ui';

	/**
	 * One project — Screen / Public project 82:735 and
	 * Mobile / Public project 88:2291.
	 *
	 * The header takes the Scan measure (860) because this sits in the projects
	 * family, but the content column is 760: the same relationship the post page
	 * has, where the header is wider than the words under it.
	 *
	 * The body arrives already rendered. §03 puts `description` in the markdown
	 * class, so it becomes HTML in `+page.server.ts` like a post's content —
	 * server-side, with no parser shipped to the reader.
	 */
	let {
		author,
		project,
		bodyHtml,
		images = []
	}: {
		author: { username: string; fullName: string; avatarSrc?: string | null };
		project: {
			title: string;
			techStack: string[];
			topics: { id: string; title: string }[];
			repoUrl?: string | null;
			demoUrl?: string | null;
		};
		/** Markup, made on the server from the project's markdown description. */
		bodyHtml: string;
		/** Cover first, then screenshots, already resolved to public paths. */
		images?: { src: string; alt: string }[];
	} = $props();

	const handle = $derived(encodeURIComponent(author.username));
	const index = $derived(`/${handle}/projects`);

	/**
	 * Which image is large. The frame marks one thumbnail with a 2px accent
	 * border (82:772), and a mark that cannot move is a mark that means nothing —
	 * so the strip chooses what the big frame shows.
	 */
	let shown = $state(0);
	const hero = $derived(images[shown] ?? images[0] ?? null);
</script>

<!-- eslint-disable svelte/no-navigation-without-resolve --
	The internal hrefs are built from a username and a slug that arrive at
	runtime, already escaped by the loader; `resolve()` would encode them twice.
	The repository and demo are the author's own addresses. -->

<div class="min-h-dvh bg-arch-bg">
	<PublicHeader
		username={author.username}
		fullName={author.fullName}
		avatarSrc={author.avatarSrc ?? null}
		current="projects"
		measure="scan"
	/>

	<div class="flex justify-center px-5 pt-[18px] pb-16 md:px-8 md:pt-[30px]">
		<article class="flex w-full max-w-[760px] flex-col gap-[14px] md:gap-5">
			<a href={index} class="text-[12px] text-arch-accent-ink hover:underline">← Projects</a>

			<h1
				class="font-display text-[27px] leading-[33px] font-extrabold text-arch-headline
				       md:text-[36px] md:leading-[42px]"
			>
				{project.title}
			</h1>

			<!-- The measure stays at 700 while the column is 760, as on a post. -->
			<div class="project-body max-w-[700px]">
				<!-- eslint-disable svelte/no-at-html-tags --
					Made by `shared/lib/markdown.server.ts`, which renders Markdown and
					drops raw HTML. The same exception the post body documents. -->
				{@html bodyHtml}
			</div>

			{#if project.techStack.length}
				<ul class="flex list-none flex-wrap gap-1.5 p-0 md:gap-[7px]">
					{#each project.techStack as tech (tech)}
						<li
							class="rounded-full bg-arch-surface-2 px-2.5 py-[5px] text-[11px] leading-[1.2]
							       text-arch-headline"
						>
							{tech}
						</li>
					{/each}
				</ul>
			{/if}

			{#if project.repoUrl || project.demoUrl}
				<!-- 18:38: one primary action per screen. The repository is what an
				     engineer came for; the demo is the second thing. -->
				<div class="flex flex-col gap-2 md:flex-row md:gap-[9px]">
					{#if project.repoUrl}
						<Button label="View repository" href={project.repoUrl} />
					{/if}
					{#if project.demoUrl}
						<Button kind="secondary" label="Live demo" href={project.demoUrl} />
					{/if}
				</div>
			{/if}

			{#if hero}
				<img
					src={hero.src}
					alt={hero.alt}
					class="h-[210px] w-full rounded-[11px] object-cover md:h-[300px] md:rounded-xl"
				/>
			{/if}

			{#if images.length > 1}
				<ul class="flex list-none gap-[9px] overflow-x-auto p-0 md:gap-3">
					{#each images as image, index (image.src)}
						<li>
							<button
								type="button"
								aria-label="Show {image.alt || `image ${index + 1}`}"
								aria-current={index === shown ? 'true' : undefined}
								onclick={() => (shown = index)}
								class="block h-[68px] w-[110px] shrink-0 overflow-hidden rounded-lg md:h-24
								       md:w-[158px] md:rounded-[9px]
								       {index === shown ? 'border-2 border-arch-accent-ink' : ''}"
							>
								<img src={image.src} alt="" class="size-full object-cover" />
							</button>
						</li>
					{/each}
				</ul>
			{/if}

			{#if project.topics.length}
				<div class="h-px bg-arch-line" role="presentation"></div>

				<div class="flex flex-wrap items-center gap-2">
					<span class="text-[11.5px] text-arch-muted">Topics</span>
					{#each project.topics as topic (topic.id)}
						<a
							href="{index}?topic_id={encodeURIComponent(topic.id)}"
							class="rounded-full border border-arch-line px-[11px] py-[5px] text-[11.5px]
							       leading-[1.2] text-arch-accent-ink hover:border-arch-line-strong"
						>
							{topic.title}
						</a>
					{/each}
				</div>
			{/if}
		</article>
	</div>
</div>

<style>
	/* The rendered body carries no Svelte scope class — it was made on the
	   server — so its elements are styled through :global from this wrapper.
	   Muted here rather than headline: on this screen the prose is the summary
	   under a title, not the article itself. */
	.project-body {
		display: flex;
		flex-direction: column;
		gap: 14px;
		color: var(--arch-muted);
		font-size: 14px;
		line-height: 23px;
	}

	@media (min-width: 48rem) {
		.project-body {
			font-size: 15.5px;
			line-height: 26px;
		}
	}

	.project-body :global(h2),
	.project-body :global(h3) {
		font-family: var(--font-display);
		font-weight: 800;
		color: var(--arch-headline);
		line-height: 1.25;
	}

	.project-body :global(h2) {
		font-size: 20px;
	}

	.project-body :global(h3) {
		font-size: 17px;
	}

	.project-body :global(a) {
		color: var(--arch-accent-ink);
		text-decoration: underline;
	}

	.project-body :global(ul),
	.project-body :global(ol) {
		padding-left: 22px;
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.project-body :global(ul) {
		list-style: disc;
	}

	.project-body :global(ol) {
		list-style: decimal;
	}

	.project-body :global(img) {
		max-width: 100%;
		border-radius: 10px;
	}

	.project-body :global(code) {
		font-family: var(--font-mono);
		font-size: 0.9em;
	}

	.project-body :global(pre) {
		background: var(--arch-surface-2);
		border-radius: 8px;
		padding: 14px;
		overflow-x: auto;
	}
</style>
