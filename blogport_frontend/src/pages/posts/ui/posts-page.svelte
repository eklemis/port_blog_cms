<script lang="ts">
	import { untrack } from 'svelte';
	import { Plus, Search } from '@lucide/svelte';
	import { Button, EmptyState, SkeletonRows, StatusPill } from '$lib/shared/ui';
	import { postStatus, updatedLabel } from '$lib/entities/post';
	import { CONSOLE_ROUTES } from '$lib/shared/config/routes';

	/**
	 * `/studio/posts`.
	 *
	 * Four states, always: loading is skeleton rows shaped like real rows rather
	 * than a centred spinner, empty says why and offers the one action that
	 * fixes it, filtered-empty is distinct from empty because conflating them
	 * tells someone with 24 posts that they have none, and error says what
	 * failed and that their work is safe.
	 *
	 * The TOPICS column the frame draws is not here: `BlogPostCardResponse`
	 * carries no topics, and filling it would be one request per row. Same for
	 * the ARCHIVED pill — see the PR. The topic *filter* is a different thing
	 * and it is here: `topic_id` was implemented all along.
	 *
	 * Design: Screen / Posts list 11:2 and its four state frames.
	 */
	type Row = { id: string; title: string; published_at?: string | null; updated_at: string };
	type Topic = { id: string; title: string };

	let {
		posts,
		topics,
		total,
		page,
		perPage,
		filtered,
		failed = false,
		loading = false,
		search = '',
		published = null,
		topic = null,
		sort = null,
		onquery
	}: {
		posts: Row[];
		/** The filter's options. Empty means the request for them failed. */
		topics: Topic[];
		total: number;
		page: number;
		perPage: number;
		filtered: boolean;
		failed?: boolean;
		loading?: boolean;
		search?: string;
		published?: string | null;
		topic?: string | null;
		sort?: string | null;
		/** Every change writes to the URL; the caller decides how. */
		onquery: (changes: Record<string, string | null>) => void;
	} = $props();

	// `untrack` because these are seeds, not bindings: the effect below is what
	// keeps them in step with the URL afterwards.
	let term = $state(untrack(() => search));
	/** The last term this box put in the URL, so its own echo is not a change. */
	let sent = $state(untrack(() => search));
	let debounce: ReturnType<typeof setTimeout> | undefined;

	$effect(() => () => clearTimeout(debounce));

	// The URL is the source of truth. Back, forward and Clear filters all change
	// `search` underneath the box and it has to follow — but not when the change
	// is the box's own debounced write coming back, which would revert whatever
	// was typed in the meantime.
	$effect(() => {
		if (search !== sent) {
			term = search;
			sent = search;
		}
	});

	function typeSearch(value: string) {
		term = value;
		clearTimeout(debounce);
		// 300ms, per the blueprint: a request per character is a rate limit
		// waiting to happen.
		debounce = setTimeout(() => {
			sent = value.trim();
			onquery({ search: sent || null, page: null });
		}, 300);
	}

	const lastPage = $derived(Math.max(1, Math.ceil(total / perPage)));
	const showing = $derived(posts.length);
</script>

<div class="flex flex-col gap-5">
	<div class="flex items-center justify-between gap-4">
		<h1
			class="font-display text-[24px] font-extrabold tracking-tight text-arch-headline md:text-[27px]"
		>
			Posts
		</h1>
		<Button label="New post" href={`${CONSOLE_ROUTES.posts}/new`}>
			{#snippet icon()}<Plus size={15} aria-hidden="true" />{/snippet}
		</Button>
	</div>

	<!-- Search left, filters centre, sort right — the same bar on every list. -->
	<div class="flex flex-wrap items-center gap-2.5">
		<div
			class="flex h-[38px] w-full items-center gap-2.5 rounded-lg border border-arch-line
			       bg-arch-surface px-3 md:w-[280px]"
		>
			<Search size={16} aria-hidden="true" class="shrink-0 text-arch-muted" />
			<input
				type="search"
				aria-label="Search posts"
				placeholder="Search posts…"
				value={term}
				oninput={(event) => typeSearch(event.currentTarget.value)}
				class="w-full min-w-0 bg-transparent text-[13px] text-arch-headline
				       placeholder:text-arch-muted focus:outline-none"
			/>
		</div>

		<label class="sr-only" for="posts-published">Show</label>
		<select
			id="posts-published"
			value={published ?? ''}
			onchange={(event) => onquery({ published: event.currentTarget.value || null, page: null })}
			class="h-[38px] rounded-lg border border-arch-line-control bg-arch-surface px-3
			       text-[13px] font-semibold text-arch-headline"
		>
			<option value="">Drafts &amp; published</option>
			<option value="true">Published</option>
			<option value="false">Drafts only</option>
		</select>

		<!-- No options means the request for them failed. A select with nothing
		     in it is a worse answer than no control at all. -->
		{#if topics.length > 0}
			<label class="sr-only" for="posts-topic">Topic</label>
			<select
				id="posts-topic"
				value={topic ?? ''}
				onchange={(event) => onquery({ topic_id: event.currentTarget.value || null, page: null })}
				class="h-[38px] rounded-lg border border-arch-line-control bg-arch-surface px-3
				       text-[13px] font-semibold text-arch-headline"
			>
				<option value="">All topics</option>
				{#each topics as option (option.id)}
					<option value={option.id}>{option.title}</option>
				{/each}
			</select>
		{/if}

		<label class="sr-only" for="posts-sort">Sort by</label>
		<select
			id="posts-sort"
			value={sort ?? 'published_newest'}
			onchange={(event) => onquery({ sort: event.currentTarget.value, page: null })}
			class="ml-auto h-[38px] rounded-lg bg-transparent px-3 text-[13px] font-semibold
			       text-arch-muted"
		>
			<option value="published_newest">Recently published</option>
			<option value="updated_newest">Recently updated</option>
			<option value="newest">Newest</option>
			<option value="oldest">Oldest</option>
		</select>
	</div>

	{#if failed}
		<!-- Never blames the person; always says their work is safe. -->
		<EmptyState
			title="We couldn't load your posts."
			message="Nothing has happened to them. Try again in a moment."
		>
			{#snippet action()}
				<Button kind="secondary" label="Try again" onclick={() => onquery({})} />
			{/snippet}
		</EmptyState>
	{:else if loading}
		<SkeletonRows label="Loading posts" />
	{:else if posts.length === 0 && filtered}
		<!-- Distinct from empty, and it says what does exist. -->
		<EmptyState
			title="No posts match those filters."
			message="You have {total === 0 ? 'posts' : `${total} posts`} in total."
		>
			{#snippet action()}
				<Button
					kind="secondary"
					label="Clear filters"
					onclick={() => {
						term = '';
						sent = '';
						onquery({ search: null, published: null, topic_id: null, page: null });
					}}
				/>
			{/snippet}
		</EmptyState>
	{:else if posts.length === 0}
		<!-- Why it is empty, and the one action that fixes it. -->
		<EmptyState title="No posts yet." message="This is where everything you write will live.">
			{#snippet action()}
				<Button label="Write your first post" href={`${CONSOLE_ROUTES.posts}/new`} />
			{/snippet}
		</EmptyState>
	{:else}
		<div class="overflow-x-auto rounded-xl border border-arch-line bg-arch-surface">
			<table class="w-full min-w-[520px] border-collapse text-left">
				<thead>
					<tr class="border-b border-arch-line">
						{#each ['Title', 'Status', 'Updated'] as heading (heading)}
							<th
								scope="col"
								class="px-[18px] py-3 font-mono text-[9px] font-normal tracking-[0.9px]
								       text-arch-muted uppercase"
							>
								{heading}
							</th>
						{/each}
					</tr>
				</thead>
				<tbody>
					{#each posts as post (post.id)}
						{@const status = postStatus(post.published_at)}
						<tr class="border-b border-arch-line last:border-b-0">
							<td class="px-[18px] py-3 text-[13px] font-medium text-arch-headline">
								{post.title}
							</td>
							<td class="px-[18px] py-3">
								<StatusPill tone={status.tone} label={status.label} />
							</td>
							<td class="px-[18px] py-3 text-[12px] text-arch-muted">
								<time datetime={post.updated_at}>{updatedLabel(post.updated_at)}</time>
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>

		<div class="flex items-center justify-between text-[12px] text-arch-muted">
			<p>{showing} of {total} posts</p>
			{#if lastPage > 1}
				<div class="flex items-center gap-1">
					<Button
						kind="ghost"
						label="Previous"
						disabled={page <= 1}
						disabledReason={page <= 1 ? 'You are on the first page.' : undefined}
						onclick={() => onquery({ page: String(page - 1) })}
					/>
					<span class="font-mono">{page} / {lastPage}</span>
					<Button
						kind="ghost"
						label="Next"
						disabled={page >= lastPage}
						disabledReason={page >= lastPage ? 'You are on the last page.' : undefined}
						onclick={() => onquery({ page: String(page + 1) })}
					/>
				</div>
			{/if}
		</div>
	{/if}
</div>
