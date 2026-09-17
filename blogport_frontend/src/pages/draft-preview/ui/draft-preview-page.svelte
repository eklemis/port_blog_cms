<script lang="ts">
	/**
	 * `/preview/[token]` — a draft as a reviewer reads it.
	 *
	 * Screen / Public post's layout with a strip on top. The strip is the point:
	 * the payload carries `preview: true` precisely so this can be rendered from
	 * the data rather than from which URL happened to be called — "without it a
	 * reviewer cannot tell a draft from the live post, and may link to it as
	 * though it were public".
	 *
	 * The public post surface proper — the author's nav, the cover image, the
	 * read time, and Markdown rendered to HTML on the server — is the public
	 * reader slice, step 8 of the build order. This shows the words as they were
	 * written: paragraphs stay paragraphs, and no Markdown is interpreted yet.
	 */
	type Preview = {
		title: string;
		excerpt?: string | null;
		content: string;
		topics?: { id: string; title: string }[];
	};

	let { post }: { post: Preview } = $props();

	/** Blank lines separate paragraphs, which is as far as this goes for now. */
	const paragraphs = $derived(post.content.split(/\n{2,}/).filter((part) => part.trim()));
</script>

<div class="min-h-screen bg-arch-bg">
	<p
		class="flex flex-wrap items-center justify-center gap-2 bg-st-inflight/12 px-4 py-2.5 text-center
		       text-[12.5px] text-arch-headline"
	>
		<span class="font-mono text-[9px] tracking-[0.9px] text-st-inflight uppercase">
			Draft preview
		</span>
		This post is not published. Anyone with this link can read it.
	</p>

	<article class="mx-auto flex max-w-[620px] flex-col gap-4 px-5 py-10">
		<h1 class="font-display text-[30px] font-extrabold tracking-tight text-arch-headline">
			{post.title}
		</h1>

		{#if post.excerpt}
			<p class="text-[15px] text-arch-muted">{post.excerpt}</p>
		{/if}

		{#if post.topics?.length}
			<ul class="flex flex-wrap gap-2">
				{#each post.topics as topic (topic.id)}
					<li
						class="rounded-full border border-arch-line px-2.5 py-1 text-[11px] text-arch-accent-ink"
					>
						{topic.title}
					</li>
				{/each}
			</ul>
		{/if}

		<div data-body class="flex flex-col gap-4">
			{#each paragraphs as paragraph, index (index)}
				<p class="text-[15px] leading-[26px] whitespace-pre-line text-arch-headline">
					{paragraph}
				</p>
			{/each}
		</div>
	</article>
</div>
