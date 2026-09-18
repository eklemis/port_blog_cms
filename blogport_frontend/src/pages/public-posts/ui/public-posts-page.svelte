<script lang="ts">
	import { PublicHeader } from '$lib/widgets/public-header';
	import { Pager } from '$lib/widgets/pager';

	/**
	 * An author's published posts — Screen / Public author index 72:66 and
	 * Mobile / Public writing 88:2163.
	 *
	 * The column is 760 here, not the post page's 700: a list of titles reads
	 * wider than an article does, and the two measures are deliberate.
	 *
	 * Nothing on this page knows the console exists (§03).
	 *
	 * **The filter row is partly here.** The frame draws every topic the author
	 * uses, as chips to filter by. There is no public endpoint that lists them —
	 * `GET /api/topics` is the *authenticated* user's own — and the posts on one
	 * page only carry the topics of those posts, which would make the row change
	 * as you page and collapse to a single chip once filtered. So the discovery
	 * row is not built and is reported; what is built is the active filter, whose
	 * title comes off the posts it returned, and the way back out of it.
	 */
	let {
		author,
		posts,
		total,
		page,
		perPage,
		filter = null,
		onpage = () => {}
	}: {
		author: { username: string; fullName: string; bio?: string | null; avatarSrc?: string | null };
		posts: {
			slug: string;
			title: string;
			excerpt?: string | null;
			/** Already spelled by `listedLabel`. */
			published: string | null;
			publishedAt?: string | null;
		}[];
		total: number;
		page: number;
		perPage: number;
		/** The topic being filtered by, when there is one. */
		filter?: { id: string; title: string } | null;
		onpage?: (page: number) => void;
	} = $props();

	const handle = $derived(encodeURIComponent(author.username));
	const index = $derived(`/${handle}/blog`);
</script>

<!-- eslint-disable svelte/no-navigation-without-resolve --
	Every href here is built from a username and a slug that arrive at runtime.
	`resolve()` takes a route id and its parameters, and these are already the
	escaped strings it would produce; routing them through it would double-encode
	what the loader has escaped once. -->

<div class="min-h-dvh bg-arch-bg">
	<PublicHeader
		username={author.username}
		fullName={author.fullName}
		avatarSrc={author.avatarSrc ?? null}
	/>

	<div class="flex justify-center px-5 pt-[18px] pb-16 md:px-6 md:pt-[34px]">
		<div class="flex w-full max-w-[760px] flex-col gap-[14px] md:gap-5">
			<!-- 72:81 — avatar, name and bio. Mobile drops all three: the header bar
			     above already says whose page this is, so the frame puts the section
			     name there instead. Only one of the two words is ever drawn, so the
			     heading reads correctly at both widths. -->
			<div class="flex items-center gap-4">
				{#if author.avatarSrc}
					<img
						src={author.avatarSrc}
						alt=""
						width="58"
						height="58"
						class="size-[58px] shrink-0 rounded-full object-cover max-md:hidden"
					/>
				{/if}
				<div class="flex min-w-0 flex-1 flex-col gap-1">
					<h1 class="font-display text-[28px] font-extrabold text-arch-headline">
						<span data-testid="identity" class="max-md:hidden">{author.fullName}</span>
						<span data-testid="section" class="md:hidden">Writing</span>
					</h1>
					{#if author.bio}
						<p class="max-w-[540px] text-[14px] text-arch-muted max-md:hidden">{author.bio}</p>
					{/if}
				</div>
			</div>

			<div class="h-px bg-arch-line max-md:hidden" role="presentation"></div>

			{#if filter}
				<!-- 72:88 — "Filter" and the chip that is on, carrying the × that
				     takes it off. Below md the label goes and the chip row scrolls
				     rather than wrapping (88:2178). -->
				<div class="flex items-center gap-2 overflow-x-auto">
					<span class="text-[11.5px] text-arch-muted max-md:hidden">Filter</span>
					<a
						href={index}
						aria-label="Clear the {filter.title} filter"
						class="flex shrink-0 items-center gap-1.5 rounded-full border border-arch-accent-ink
						       bg-arch-surface-2 px-[11px] py-[5px] text-[11.5px] leading-[1.2]
						       font-semibold text-arch-accent-ink md:py-[6px]"
					>
						{filter.title}
						<span aria-hidden="true" class="font-normal">×</span>
					</a>
				</div>
			{/if}

			{#if posts.length}
				<ul class="flex list-none flex-col p-0">
					{#each posts as post (post.slug)}
						<!-- The rule sits under every row, the last one included: 72:114 draws
						     one between the final post and the count. -->
						<li class="flex flex-col gap-1.5 border-b border-arch-line py-3.5 md:gap-[7px] md:py-4">
							{#if post.published}
								<p class="font-mono text-[10.5px] text-arch-muted md:text-[11px]">
									<time datetime={post.publishedAt ?? undefined}>{post.published}</time>
								</p>
							{/if}
							<a
								href="{index}/{encodeURIComponent(post.slug)}"
								class="font-display text-[18px] leading-6 font-bold text-arch-headline
								       hover:text-arch-accent-ink md:text-[21px] md:leading-[1.2]"
							>
								{post.title}
							</a>
							{#if post.excerpt}
								<p
									class="max-w-[700px] text-[13.5px] leading-[21px] text-arch-muted
									       md:text-[14px] md:leading-[23px]"
								>
									{post.excerpt}
								</p>
							{/if}
						</li>
					{/each}
				</ul>

				<Pager shown={posts.length} {total} {page} {perPage} {onpage} />
			{:else}
				<!-- No document specifies this copy, because no frame draws an empty
				     public listing. Kept plain and reported rather than dressed up. -->
				<div class="flex flex-col items-start gap-2 py-10">
					<p class="text-[14px] text-arch-muted">
						{filter ? 'No posts under this topic.' : 'Nothing published yet.'}
					</p>
					{#if filter}
						<a href={index} class="text-[13px] font-semibold text-arch-accent-ink hover:underline">
							Show all posts
						</a>
					{/if}
				</div>
			{/if}
		</div>
	</div>
</div>
