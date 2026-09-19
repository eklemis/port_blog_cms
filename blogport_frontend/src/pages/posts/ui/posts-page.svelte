<script lang="ts">
	import { untrack } from 'svelte';
	import { resolve } from '$app/paths';
	import { Plus, Search } from '@lucide/svelte';
	import {
		Button,
		EmptyState,
		InlineAlert,
		Menu,
		SkeletonRows,
		StatusPill,
		Toast
	} from '$lib/shared/ui';
	import { archivePost, restorePost } from '$lib/features/manage-archive';
	import type { HandlingClass } from '$lib/shared/lib/error-class';
	import { postStatus, scheduledLabel, updatedLabel } from '$lib/entities/post';
	import { Pager } from '$lib/widgets/pager';
	import { CONSOLE_ROUTES } from '$lib/shared/config/routes';
	import { filteredSentence } from '../model/filtered-sentence';

	/**
	 * `/studio/posts`.
	 *
	 * Four states, always: loading is skeleton rows shaped like real rows rather
	 * than a centred spinner, empty says why and offers the one action that
	 * fixes it, filtered-empty is distinct from empty because conflating them
	 * tells someone with 24 posts that they have none, and error says what
	 * failed and that their work is safe.
	 *
	 * The TOPICS column reads straight from the card, which carries them now —
	 * loaded once for the page, not per row — as one line joined with a middle
	 * dot, the way Screen / Posts list 11:2 draws it. It is the column tablet
	 * drops, per the Prototype Map's table rules. The ARCHIVED pill is still not here: this
	 * list never contains an archived post.
	 *
	 * Design: Screen / Posts list 11:2 and its four state frames.
	 */
	type Row = {
		id: string;
		title: string;
		published_at?: string | null;
		updated_at: string;
		/** Always present. Empty means the post has none, never "not loaded". */
		topics: { id: string; title: string }[];
	};
	type Topic = { id: string; title: string };

	let {
		posts,
		topics,
		total,
		everything = null,
		archived = 0,
		onchanged = () => {},
		fetchFn = undefined,
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
		/**
		 * Every post in the main list, unfiltered — for the filtered-empty
		 * sentence, where `total` is the zero that brought someone there.
		 */
		everything?: number | null;
		/** How many posts are in the archive, for the link beside the count. */
		archived?: number;
		/** A post left the list — the caller reloads it. */
		onchanged?: () => void;
		/** Injected by the spec; the browser's own otherwise. */
		fetchFn?: typeof globalThis.fetch;
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

	const topicTitle = $derived(topics.find((option) => option.id === topic)?.title ?? null);

	let archiving = $state(false);
	let failure = $state<string | undefined>();
	let failureKind = $state<HandlingClass>('notOurs');
	/** The post the toast can put back, while it is still on screen. */
	let undoable = $state<{ id: string } | null>(null);

	async function archive(post: Row) {
		archiving = true;
		failure = undefined;

		const result = await archivePost(post.id, fetchFn);
		archiving = false;

		if (!result.ok) {
			failure = result.message;
			failureKind = result.kind;
			return;
		}

		// §06's first rung: one click, and eight seconds to take it back.
		undoable = { id: post.id };
		onchanged();
	}

	async function undo() {
		if (!undoable) return;

		const result = await restorePost(undoable.id, fetchFn);
		undoable = null;

		if (!result.ok) {
			failure = result.message;
			failureKind = result.kind;
			return;
		}

		onchanged();
	}

	/** A filter at rest: the secondary button's box, 13px semibold (Button 75:83). */
	const PILL =
		'appearance-none rounded-lg border border-arch-line-control bg-arch-surface px-4 py-2.5 text-[13px] font-semibold text-arch-headline';

	const statusChip = $derived(
		published === 'false' ? 'Drafts' : published === 'true' ? 'Published' : null
	);

	/**
	 * The phone's status filter. The frame's third chip reads "Live"; it says
	 * "Published" here because `published=true` includes scheduled posts, which
	 * are not live — the same reason the designer ruled "Published" on desktop.
	 */
	const STATUS_CHIPS = [
		{ label: 'All', value: '' },
		{ label: 'Drafts', value: 'false' },
		{ label: 'Published', value: 'true' }
	];

	/** "Rust · Systems · 2 days ago", or "Rust · goes live tomorrow". */
	function meta(post: Row) {
		const status = postStatus(post.published_at);
		const when =
			status.label === 'Scheduled' && post.published_at
				? scheduledLabel(post.published_at)
				: updatedLabel(post.updated_at);
		return [...post.topics.map((item) => item.title), when].join(' · ');
	}

	const showing = $derived(posts.length);
</script>

<div class="flex flex-col gap-5">
	<div class="flex items-center justify-between gap-4">
		<!-- On a phone the shell's header names the screen and carries New, so the
		     page's own title is kept for screen readers and nothing else. -->
		<h1
			class="font-display text-[24px] font-extrabold tracking-tight text-arch-headline max-md:sr-only
			       md:text-[27px]"
		>
			Posts
		</h1>
		<div class="flex items-center gap-2 max-md:hidden">
			<Button label="New post" href={`${CONSOLE_ROUTES.posts}/new`}>
				{#snippet icon()}<Plus size={15} aria-hidden="true" />{/snippet}
			</Button>
		</div>
	</div>

	<!--
		Search left, filters centre, sort right — the same bar on every list. Not
		drawn when there are no posts at all: Screen / Posts — empty hides it,
		because controls that cannot do anything are noise. It stays for the error
		and filtered states, where the query is the thing worth keeping.
	-->
	{#if !(posts.length === 0 && !filtered && !failed && !loading)}
		<div class="flex flex-wrap items-center gap-2.5 max-md:gap-3">
			<!-- The search is narrower between md and lg. Tablet / Posts list 166:5098
			     keeps all four controls on one row, and at 834 the icon rail leaves
			     710 for a toolbar that wants 726 at the desktop width. -->
			<div
				class="flex h-10 w-full items-center gap-[9px] rounded-[9px] border border-arch-line
				       bg-arch-surface px-[13px] md:h-[38px] md:w-[240px] md:rounded-lg lg:w-[280px]"
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

			<!-- Phone: three chips, one on. Mobile / Posts list 73:10. -->
			<div role="group" aria-label="Show" class="flex gap-[7px] md:hidden">
				{#each STATUS_CHIPS as chip (chip.label)}
					{@const on = (published ?? '') === chip.value}
					<button
						type="button"
						aria-pressed={on}
						onclick={() => onquery({ published: chip.value || null, page: null })}
						class="rounded-full border px-3 py-1.5 text-[11.5px]
						       {on
							? 'border-arch-accent-ink bg-arch-surface-2 font-semibold text-arch-accent-ink'
							: 'border-arch-line text-arch-muted'}"
					>
						{chip.label}
					</button>
				{/each}
			</div>

			<!--
				From 768px: a filter at rest looks like a secondary button, and an
				active one becomes a chip that takes it off — Screen / Posts — filtered
				empty 84:557. Native selects underneath, so choosing keeps the
				platform's own list and its keyboard behaviour.
			-->
			{#if topics.length > 0}
				{#if topicTitle}
					<button
						type="button"
						aria-label="{topicTitle}, remove filter"
						onclick={() => onquery({ topic_id: null, page: null })}
						class="flex items-center gap-1.5 rounded-full border border-arch-accent-ink
						       bg-arch-surface-2 px-3 py-2 text-arch-accent-ink max-md:hidden"
					>
						<span class="text-[11.5px] font-semibold">{topicTitle}</span>
						<span aria-hidden="true" class="text-[11px]">×</span>
					</button>
				{:else}
					<label class="sr-only" for="posts-topic">Topic</label>
					<select
						id="posts-topic"
						value=""
						onchange={(event) =>
							onquery({ topic_id: event.currentTarget.value || null, page: null })}
						class="{PILL} max-md:hidden"
					>
						<option value="">All topics</option>
						{#each topics as option (option.id)}
							<option value={option.id}>{option.title}</option>
						{/each}
					</select>
				{/if}
			{/if}

			{#if statusChip}
				<button
					type="button"
					aria-label="{statusChip}, remove filter"
					onclick={() => onquery({ published: null, page: null })}
					class="flex items-center gap-1.5 rounded-full border border-arch-accent-ink
					       bg-arch-surface-2 px-3 py-2 text-arch-accent-ink max-md:hidden"
				>
					<span class="text-[11.5px] font-semibold">{statusChip}</span>
					<span aria-hidden="true" class="text-[11px]">×</span>
				</button>
			{:else}
				<label class="sr-only" for="posts-published">Show</label>
				<select
					id="posts-published"
					value=""
					onchange={(event) =>
						onquery({ published: event.currentTarget.value || null, page: null })}
					class="{PILL} max-md:hidden"
				>
					<option value="">Drafts &amp; published</option>
					<option value="true">Published</option>
					<option value="false">Drafts</option>
				</select>
			{/if}

			<label class="sr-only" for="posts-sort">Sort by</label>
			<select
				id="posts-sort"
				value={sort ?? 'published_newest'}
				onchange={(event) => onquery({ sort: event.currentTarget.value, page: null })}
				class="appearance-none rounded-lg bg-transparent px-4 py-2.5 text-[13px] font-semibold
				       text-arch-muted max-md:hidden"
			>
				<option value="published_newest">Recently published</option>
				<option value="updated_newest">Recently updated</option>
				<option value="newest">Newest</option>
				<option value="oldest">Oldest</option>
			</select>
		</div>
	{/if}

	<InlineAlert message={failure} kind={failureKind} />

	{#if failed}
		<!-- CollectionState / error: never blames the person, says the work is safe. -->
		<EmptyState
			tone="danger"
			title="Couldn’t load your posts"
			message="Something went wrong on our side. Your posts are safe."
		>
			{#snippet action()}
				<Button kind="secondary" size="compact" label="Try again" onclick={() => onquery({})} />
			{/snippet}
		</EmptyState>
	{:else if loading}
		<SkeletonRows label="Loading posts" />
	{:else if posts.length === 0 && filtered}
		<!-- Distinct from empty: it says what does exist, and which filter is why. -->
		<EmptyState
			title="No posts match those filters"
			message={filteredSentence({ everything, published, topic: topicTitle, search })}
		>
			{#snippet action()}
				<Button
					kind="secondary"
					size="compact"
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
		<EmptyState
			title="No posts yet"
			message="Your first post is the one that turns this into a blog."
		>
			{#snippet action()}
				<Button size="compact" label="Write your first post" href={`${CONSOLE_ROUTES.posts}/new`} />
			{/snippet}
		</EmptyState>
	{:else}
		<div class="overflow-x-auto rounded-xl border border-arch-line bg-arch-surface max-md:hidden">
			<table class="w-full border-collapse text-left">
				<thead>
					<tr class="border-b border-arch-line">
						{#each ['Title', 'Status', 'Topics', 'Updated', 'Actions'] as heading (heading)}
							<th
								scope="col"
								class="px-[18px] py-3 font-mono text-[9px] font-normal tracking-[0.9px]
								       text-arch-muted uppercase {heading === 'Topics' ? 'hidden lg:table-cell' : ''}
								       {heading === 'Actions' ? 'sr-only' : ''}"
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
							<td class="px-[18px] py-3 text-[13px] font-medium">
								<!-- The row's own link rather than a whole-row click target: a
								     table row that navigates cannot be reached by keyboard, and
								     the title is the thing being opened. -->
								<a
									href={resolve('/studio/posts/[id]', { id: post.id })}
									class="text-arch-headline underline-offset-4 hover:underline"
								>
									{post.title}
								</a>
							</td>
							<td class="px-[18px] py-3">
								<StatusPill tone={status.tone} label={status.label} />
							</td>
							<!-- Dropped below 1024px — the middle column goes first, per the
							     Prototype Map. A dash when the post has none, as the frame draws:
							     empty on the card means none, never "not loaded". -->
							<td class="hidden px-[18px] py-3 text-[12px] text-arch-muted lg:table-cell">
								{post.topics.length ? post.topics.map((topic) => topic.title).join(' · ') : '—'}
							</td>
							<td class="px-[18px] py-3 text-[12px] text-arch-muted">
								<time datetime={post.updated_at}>{updatedLabel(post.updated_at)}</time>
							</td>
							<!-- §06: Archive is one click, so it lives behind the ⋯ and not on
							     the row. Not on the phone cards, by the same rule. -->
							<td class="w-12 px-[18px] py-3">
								<Menu label="More for {post.title}">
									{#snippet items(close)}
										<button
											type="button"
											role="menuitem"
											disabled={archiving}
											onclick={() => {
												close();
												archive(post);
											}}
											class="px-4 py-2 text-left text-[13px] text-arch-headline
											       hover:bg-arch-surface-2 disabled:opacity-55"
										>
											Archive
										</button>
									{/snippet}
								</Menu>
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>

		<!-- Below 768px: a card per post, as Mobile / Posts list 73:17 draws it. -->
		<ul class="flex flex-col gap-3 md:hidden">
			{#each posts as post (post.id)}
				{@const status = postStatus(post.published_at)}
				<li
					class="flex flex-col gap-2 rounded-[11px] border border-arch-line bg-arch-surface px-[15px]
					       py-3.5"
				>
					<a
						href={resolve('/studio/posts/[id]', { id: post.id })}
						class="text-[14.5px] leading-5 font-semibold text-arch-headline"
					>
						{post.title}
					</a>
					<div class="flex items-center gap-[9px]">
						<StatusPill tone={status.tone} label={status.label} />
						<p class="text-[10.5px] text-arch-muted">{meta(post)}</p>
					</div>
				</li>
			{/each}
		</ul>

		<Pager
			shown={showing}
			{total}
			{page}
			{perPage}
			noun="posts"
			onpage={(next) => onquery({ page: String(next) })}
		>
			{#snippet after()}
				{#if archived > 0}
					<!-- A destination, not a thing to do — so it sits by the count. -->
					<a
						href={resolve('/studio/posts/archive')}
						class="font-semibold text-arch-accent-ink hover:underline"
					>
						View archive ({archived})
					</a>
				{/if}
			{/snippet}
		</Pager>

		{#if undoable}
			<div class="fixed inset-x-4 bottom-4 z-10 md:right-6 md:left-auto md:w-[380px]">
				<Toast message="Archived." onclose={() => (undoable = null)}>
					{#snippet action()}
						<button
							type="button"
							onclick={undo}
							class="text-[12.5px] font-semibold text-arch-accent-ink hover:underline"
						>
							Undo
						</button>
						<span aria-hidden="true" class="text-arch-muted">·</span>
						<a
							href={resolve('/studio/posts/archive')}
							class="text-[12.5px] font-semibold text-arch-accent-ink hover:underline"
						>
							View archive
						</a>
					{/snippet}
				</Toast>
			</div>
		{/if}
	{/if}
</div>
