<script lang="ts">
	import { PublicHeader } from '$lib/widgets/public-header';
	import { publishedLabel } from '$lib/entities/post';

	/**
	 * A published post, as a stranger reads it — Screen / Public post 72:2 and
	 * Mobile / Public post 73:292.
	 *
	 * The body arrives already rendered. Markdown → HTML happens in
	 * `+page.server.ts` (the frame says so at 72:32), so no parser ships to the
	 * reader and nothing here has to trust the source text.
	 *
	 * Nothing on this page knows the console exists. §03: "No edit affordances,
	 * no draft indicators, no 'Sign in' nag over the content. The public reader
	 * is not a lapsed author."
	 */
	let {
		author,
		post,
		topics = [],
		cover = null,
		bodyHtml,
		readMinutes
	}: {
		author: { username: string; fullName: string; avatarSrc?: string | null };
		post: { title: string; excerpt?: string | null; publishedAt?: string | null };
		topics?: { id: string; title: string }[];
		/** Already resolved to a public media path, or `null` while it processes. */
		cover?: { src: string; alt: string } | null;
		/** Markup, made on the server. */
		bodyHtml: string;
		readMinutes: number;
	} = $props();

	const on = $derived(publishedLabel(post.publishedAt));
	const handle = $derived(encodeURIComponent(author.username));
</script>

<div class="min-h-dvh bg-arch-bg">
	<PublicHeader
		username={author.username}
		fullName={author.fullName}
		avatarSrc={author.avatarSrc ?? null}
	/>

	<!-- The gutter stays at every width: `md:px-0` would leave four pixels
	     either side of a 760px column at 768. -->
	<div class="flex justify-center px-5 pt-[18px] pb-16 md:px-6 md:pt-[34px]">
		<article class="flex w-full max-w-[700px] flex-col gap-[14px] md:gap-[18px]">
			<!-- 72:17 — the date in mono, the read time beside it. -->
			<p class="flex items-center gap-[10px] text-[10.5px] text-arch-muted md:text-[11.5px]">
				{#if on}
					<span class="font-mono">{on}</span>
					<span aria-hidden="true">·</span>
				{/if}
				<span>{readMinutes} min read</span>
			</p>

			<h1
				class="font-display text-[29px] leading-[35px] font-extrabold text-arch-headline
				       md:text-[40px] md:leading-[46px]"
			>
				{post.title}
			</h1>

			{#if post.excerpt}
				<p
					data-excerpt
					class="text-[14.5px] leading-[23px] text-arch-muted md:text-[16px] md:leading-[26px]"
				>
					{post.excerpt}
				</p>
			{/if}

			{#if topics.length}
				<!-- eslint-disable svelte/no-navigation-without-resolve --
					§03: the chips are real links — both public listings take topic_id.
					The author index is the next slice, and `resolve()` only takes a
					route id that exists. Swap it in when that route lands. -->
				<ul class="flex list-none flex-wrap gap-[7px] p-0 md:gap-2">
					{#each topics as topic (topic.id)}
						<li>
							<a
								href="/{handle}/blog?topic_id={encodeURIComponent(topic.id)}"
								class="block rounded-full border border-arch-line px-[10px] py-[4px] text-[11px]
								       leading-[1.2] text-arch-accent-ink hover:border-arch-line-strong
								       md:px-[11px] md:py-[5px] md:text-[11.5px]"
							>
								{topic.title}
							</a>
						</li>
					{/each}
				</ul>
			{/if}

			{#if cover}
				<img
					src={cover.src}
					alt={cover.alt}
					class="h-[150px] w-full rounded-[10px] object-cover md:h-[240px] md:rounded-xl"
				/>
			{/if}

			<!-- Body type goes up here, not down: 15.5px at this measure keeps the
			     line near 60 characters (73:310). -->
			<div class="post-body">
				<!-- eslint-disable svelte/no-at-html-tags --
					The one place this markup can come from is
					`shared/lib/markdown.server.ts`, which renders Markdown and drops raw
					HTML — there is no path by which stored text reaches here as markup.
					The rule is right in general and this is the exception it exists for. -->
				{@html bodyHtml}
			</div>
		</article>
	</div>
</div>

<style>
	/* The rendered body carries no Svelte scope class — it was made on the
	   server — so its elements are styled through :global from this wrapper. */
	.post-body {
		display: flex;
		flex-direction: column;
		gap: 18px;
		color: var(--arch-headline);
		font-size: 15.5px;
		line-height: 27px;
	}

	@media (min-width: 48rem) {
		.post-body {
			font-size: 16px;
			line-height: 29px;
		}
	}

	.post-body :global(h2),
	.post-body :global(h3) {
		font-family: var(--font-display);
		font-weight: 800;
		line-height: 1.25;
		color: var(--arch-headline);
	}

	.post-body :global(h2) {
		font-size: 22px;
	}

	.post-body :global(h3) {
		font-size: 18px;
	}

	.post-body :global(a) {
		color: var(--arch-accent-ink);
		text-decoration: underline;
	}

	.post-body :global(ul),
	.post-body :global(ol) {
		padding-left: 22px;
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.post-body :global(ul) {
		list-style: disc;
	}

	.post-body :global(ol) {
		list-style: decimal;
	}

	.post-body :global(img) {
		max-width: 100%;
		border-radius: 10px;
	}

	.post-body :global(blockquote) {
		border-left: 2px solid var(--arch-line-strong);
		padding-left: 14px;
		color: var(--arch-muted);
	}

	.post-body :global(code) {
		font-family: var(--font-mono);
		font-size: 0.9em;
	}

	.post-body :global(pre) {
		background: var(--arch-surface-2);
		border-radius: 8px;
		padding: 14px;
		overflow-x: auto;
	}

	.post-body :global(hr) {
		border: 0;
		border-top: 1px solid var(--arch-line);
	}
</style>
