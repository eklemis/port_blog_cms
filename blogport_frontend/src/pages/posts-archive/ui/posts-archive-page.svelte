<script lang="ts">
	import { SvelteSet } from 'svelte/reactivity';
	import {
		Button,
		ConfirmDialog,
		EmptyState,
		InlineAlert,
		SkeletonRows,
		StatusPill,
		Toast
	} from '$lib/shared/ui';
	import { bulkPosts, purgePost, restorePost } from '$lib/features/manage-archive';
	import type { HandlingClass } from '$lib/shared/lib/error-class';
	import { CONSOLE_ROUTES } from '$lib/shared/config/routes';

	/**
	 * `/studio/posts/archive` — §06's second and third rungs of destruction.
	 *
	 * Design: Screen / Posts archive 71:126 · Mobile / Posts archive 97:2744.
	 * A table with a selection bar from 768px, cards below it; per-row actions
	 * stay visible either way, "so a single restore does not require selecting
	 * first", as the mobile frame's note puts it.
	 *
	 * Restore is one click and no confirm. Purge is ConfirmDialog. A selection is
	 * acted on in one bulk call, and because a 200 there means the batch ran and
	 * not that every item did, failed rows stay selected with their own reason.
	 *
	 * Titles are not links: reading an archived post answers 404 by design.
	 */
	type Row = { id: string; title: string; updated_at: string };

	let {
		posts,
		total,
		page,
		perPage,
		failed = false,
		loading = false,
		onchanged,
		onpage = () => {},
		/** Injected by the spec; the browser's own otherwise. */
		fetchFn = undefined
	}: {
		posts: Row[];
		total: number;
		page: number;
		perPage: number;
		failed?: boolean;
		loading?: boolean;
		/** Something left the archive — the caller reloads the list. */
		onchanged: () => void;
		onpage?: (page: number) => void;
		fetchFn?: typeof globalThis.fetch;
	} = $props();

	const selected = new SvelteSet<string>();
	/** Per-row reasons from the last batch, keyed by id. */
	let reasons = $state<Record<string, string>>({});

	let busy = $state(false);
	let failure = $state<string | undefined>();
	let failureKind = $state<HandlingClass>('notOurs');
	let toast = $state<string | undefined>();

	/** What the open dialog would purge: one post by title, or the selection. */
	let purging = $state<{ ids: string[]; match: string; label: string } | null>(null);

	const lastPage = $derived(Math.max(1, Math.ceil(total / perPage)));
	const count = $derived(selected.size);

	/** "Restore both" for two, as the frame draws it; "Restore all 3" past that. */
	function many(verb: string, n: number) {
		if (n === 1) return verb;
		if (n === 2) return `${verb} both`;
		return `${verb} all ${n}`;
	}

	/**
	 * When it was archived. Archiving stamps `updated_at`, and an archived post
	 * cannot be edited, so for these rows the last update is the archiving.
	 */
	const month = new Intl.DateTimeFormat(undefined, { month: 'short', year: 'numeric' });
	function archivedOn(iso: string) {
		const when = new Date(iso);
		return Number.isNaN(when.getTime()) ? '—' : month.format(when);
	}

	function toggle(id: string) {
		if (selected.has(id)) selected.delete(id);
		else selected.add(id);
	}

	function report(message: string, kind: HandlingClass) {
		failure = message;
		failureKind = kind;
	}

	async function restoreOne(post: Row) {
		busy = true;
		failure = undefined;

		const result = await restorePost(post.id, fetchFn);
		busy = false;

		if (!result.ok) return report(result.message, result.kind);

		selected.delete(post.id);
		// A toast, because the post went somewhere this screen cannot show.
		toast = 'Restored. It is back in your posts.';
		onchanged();
	}

	/** Applies a batch result: failures stay selected, with their reason. */
	function settle(succeeded: string[], failures: Record<string, string>) {
		for (const id of succeeded) selected.delete(id);
		reasons = failures;
		if (succeeded.length) onchanged();
	}

	async function restoreSelected() {
		busy = true;
		failure = undefined;
		const ids = [...selected];

		const result = await bulkPosts('restore', ids, fetchFn);
		busy = false;

		if (!result.ok) return report(result.message, result.kind);

		settle(result.succeeded, result.failed);
		if (result.succeeded.length) {
			toast =
				result.succeeded.length === 1
					? 'Restored. It is back in your posts.'
					: `Restored ${result.succeeded.length}. They are back in your posts.`;
		}
	}

	async function purge() {
		if (!purging) return;

		busy = true;
		failure = undefined;
		const { ids } = purging;

		if (ids.length === 1 && !selected.has(ids[0])) {
			const result = await purgePost(ids[0], fetchFn);
			busy = false;
			// The dialog stays open on failure: the post and the decision are
			// both still here.
			if (!result.ok) return report(result.message, result.kind);
			purging = null;
			onchanged();
			return;
		}

		const result = await bulkPosts('hard_delete', ids, fetchFn);
		busy = false;

		if (!result.ok) return report(result.message, result.kind);

		purging = null;
		settle(result.succeeded, result.failed);
	}

	function askToPurge(post: Row) {
		failure = undefined;
		purging = { ids: [post.id], match: post.title, label: 'Purge' };
	}

	function askToPurgeSelected() {
		failure = undefined;
		// No single title can stand for several posts, so the count is typed —
		// the thing someone could get wrong by selecting one row too many.
		purging = { ids: [...selected], match: `${count} posts`, label: many('Purge', count) };
	}
</script>

<div class="flex flex-col gap-4">
	<div class="flex items-start justify-between gap-4">
		<div class="flex items-center gap-2.5">
			<h1
				class="font-display text-[24px] font-extrabold tracking-tight text-arch-headline md:text-[27px]"
			>
				Archive
			</h1>
			<StatusPill tone="dormant" label="Archived" />
		</div>
		<Button kind="secondary" label="Back to posts" href={CONSOLE_ROUTES.posts} />
	</div>

	<p class="text-[12.5px] text-arch-muted">
		Archived posts are not public and not in your main list. Restore puts one back in whatever state
		it was in; purge is permanent and asks you to type the title.
	</p>

	{#if !purging}
		<InlineAlert message={failure} kind={failureKind} />
	{/if}

	{#if count > 0}
		<!-- The selection bar. It replaces the header row on a phone. -->
		<div
			class="flex flex-wrap items-center gap-2.5 rounded-xl border border-arch-accent-ink
			       bg-arch-surface-2 px-4 py-2.5"
		>
			<p class="text-[13px] font-semibold text-arch-headline">{count} selected</p>
			<div class="flex gap-2 max-md:ml-auto">
				<Button
					kind="secondary"
					label={many('Restore', count)}
					disabled={busy}
					onclick={restoreSelected}
				/>
				<Button
					kind="danger"
					label={many('Purge', count)}
					disabled={busy}
					onclick={askToPurgeSelected}
				/>
			</div>
		</div>
	{/if}

	{#if failed}
		<EmptyState
			title="We couldn't load your archive."
			message="Nothing has happened to those posts. Try again in a moment."
		/>
	{:else if loading}
		<SkeletonRows label="Loading archived posts" />
	{:else if posts.length === 0}
		<EmptyState
			title="Nothing archived."
			message="Posts you archive wait here until you restore them or purge them for good."
		/>
	{:else}
		{#snippet actions(post: Row)}
			<!-- The visible word is the verb; the accessible name says which post,
			     because a column of identical "Restore" buttons is a list nobody
			     can tell apart. -->
			<button
				type="button"
				aria-label="Restore {post.title}"
				disabled={busy}
				onclick={() => restoreOne(post)}
				class="min-h-9 px-1.5 text-[12.5px] font-semibold text-arch-accent-ink disabled:opacity-55"
			>
				Restore
			</button>
			<button
				type="button"
				aria-label="Purge {post.title}"
				disabled={busy}
				onclick={() => askToPurge(post)}
				class="min-h-9 px-1.5 text-[12.5px] font-semibold text-st-danger disabled:opacity-55"
			>
				Purge
			</button>
		{/snippet}

		{#snippet check(post: Row)}
			<input
				type="checkbox"
				aria-label="Select {post.title}"
				checked={selected.has(post.id)}
				onchange={() => toggle(post.id)}
				class="size-4 accent-arch-accent-ink"
			/>
		{/snippet}

		<!-- From 768px: the table the desktop frame draws. -->
		<div class="overflow-x-auto rounded-xl border border-arch-line bg-arch-surface max-md:hidden">
			<table class="w-full border-collapse text-left">
				<thead>
					<tr class="border-b border-arch-line">
						<th scope="col" class="w-12 px-[18px] py-3"><span class="sr-only">Select</span></th>
						{#each ['Title', 'Archived'] as heading (heading)}
							<th
								scope="col"
								class="px-[18px] py-3 font-mono text-[9px] font-normal tracking-[0.9px]
								       text-arch-muted uppercase"
							>
								{heading}
							</th>
						{/each}
						<th scope="col" class="px-[18px] py-3"><span class="sr-only">Actions</span></th>
					</tr>
				</thead>
				<tbody>
					{#each posts as post (post.id)}
						<tr class="border-b border-arch-line last:border-b-0">
							<td class="px-[18px] py-3">{@render check(post)}</td>
							<td class="px-[18px] py-3">
								<p class="text-[13px] font-medium text-arch-headline">{post.title}</p>
								{#if reasons[post.id] && selected.has(post.id)}
									<p class="text-[11.5px] text-st-danger">{reasons[post.id]}</p>
								{/if}
							</td>
							<td class="px-[18px] py-3 text-[12px] text-arch-muted">
								<time datetime={post.updated_at}>{archivedOn(post.updated_at)}</time>
							</td>
							<td class="px-[18px] py-3">
								<div class="flex gap-2">{@render actions(post)}</div>
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>

		<!-- Below 768px: cards, as Mobile / Posts archive draws them. -->
		<ul class="flex flex-col gap-2.5 md:hidden">
			{#each posts as post (post.id)}
				<li
					class="flex flex-col gap-2 rounded-xl border bg-arch-surface p-3.5
					       {selected.has(post.id) ? 'border-arch-accent-ink' : 'border-arch-line'}"
				>
					<div class="flex items-center gap-2.5">
						{@render check(post)}
						<p class="text-[14px] font-medium text-arch-headline">{post.title}</p>
					</div>
					{#if reasons[post.id] && selected.has(post.id)}
						<p class="text-[11.5px] text-st-danger">{reasons[post.id]}</p>
					{/if}
					<div class="flex items-center justify-between">
						<p class="font-mono text-[11.5px] text-arch-muted">
							archived <time datetime={post.updated_at}>{archivedOn(post.updated_at)}</time>
						</p>
						<div class="flex gap-1">{@render actions(post)}</div>
					</div>
				</li>
			{/each}
		</ul>

		{#if lastPage > 1}
			<div class="flex items-center justify-end gap-1 text-[12px] text-arch-muted">
				<Button
					kind="ghost"
					label="Previous"
					disabled={page <= 1}
					disabledReason={page <= 1 ? 'You are on the first page.' : undefined}
					onclick={() => onpage(page - 1)}
				/>
				<span class="font-mono">{page} / {lastPage}</span>
				<Button
					kind="ghost"
					label="Next"
					disabled={page >= lastPage}
					disabledReason={page >= lastPage ? 'You are on the last page.' : undefined}
					onclick={() => onpage(page + 1)}
				/>
			</div>
		{/if}
	{/if}
</div>

<ConfirmDialog
	open={purging !== null}
	title={purging && purging.ids.length > 1
		? `Purge ${purging.ids.length} posts forever?`
		: 'Purge this post forever?'}
	consequence="It cannot be restored. Anyone with a link to it will find nothing there."
	match={purging?.match ?? ''}
	confirmLabel={purging?.label ?? 'Purge'}
	working={busy}
	failure={purging ? failure : undefined}
	onconfirm={purge}
	oncancel={() => (purging = null)}
/>

{#if toast}
	<div class="fixed inset-x-4 bottom-4 z-10 md:right-6 md:left-auto md:w-[380px]">
		<Toast message={toast} onclose={() => (toast = undefined)} />
	</div>
{/if}
