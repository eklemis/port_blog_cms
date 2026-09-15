<script lang="ts">
	import {
		Button,
		ConfirmDialog,
		EmptyState,
		InlineAlert,
		SkeletonRows,
		StatusPill,
		Toast
	} from '$lib/shared/ui';
	import { updatedLabel } from '$lib/entities/post';
	import { purgePost, restorePost } from '$lib/features/manage-archive';
	import type { HandlingClass } from '$lib/shared/lib/error-class';
	import { CONSOLE_ROUTES } from '$lib/shared/config/routes';

	/**
	 * `/studio/posts/archive` — where §06's second and third rungs of
	 * destruction live.
	 *
	 * Restore is one click and no confirm. Delete forever is a dialog naming the
	 * post and asking for its title, with a danger button. Neither is
	 * optimistic: the list is reloaded from the server once the call lands.
	 *
	 * Titles are not links. Opening an archived post reads it through
	 * `GET /api/blog/{id}`, which answers 404 for an archived post by design —
	 * so a link here would lead only to "You don't have access".
	 *
	 * Design: Screen / Archived posts.
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

	let restoring = $state<string | null>(null);
	let purging = $state<Row | null>(null);
	let working = $state(false);
	let failure = $state<string | undefined>();
	let failureKind = $state<HandlingClass>('notOurs');
	let toast = $state<string | undefined>();

	const lastPage = $derived(Math.max(1, Math.ceil(total / perPage)));

	async function restore(post: Row) {
		restoring = post.id;
		failure = undefined;

		const result = await restorePost(post.id, fetchFn);
		restoring = null;

		if (!result.ok) {
			failure = result.message;
			failureKind = result.kind;
			return;
		}

		// A toast, because the post went somewhere this screen cannot show.
		toast = 'Restored. It is back in your posts.';
		onchanged();
	}

	async function purge() {
		if (!purging) return;

		working = true;
		failure = undefined;

		const result = await purgePost(purging.id, fetchFn);
		working = false;

		if (!result.ok) {
			// The dialog stays open: closing it would read as done, and the post
			// and the decision are both still here.
			failure = result.message;
			failureKind = result.kind;
			return;
		}

		purging = null;
		onchanged();
	}
</script>

<div class="flex flex-col gap-5">
	<div class="flex flex-col gap-1.5">
		<h1
			class="font-display text-[24px] font-extrabold tracking-tight text-arch-headline md:text-[27px]"
		>
			Archived posts
		</h1>
		<p class="text-[13px] text-arch-muted">
			Out of the way, and recoverable until you delete them forever.
		</p>
	</div>

	{#if !purging}
		<InlineAlert message={failure} kind={failureKind} />
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
			message="Posts you archive wait here until you restore them or delete them for good."
		>
			{#snippet action()}
				<Button kind="secondary" label="Back to posts" href={CONSOLE_ROUTES.posts} />
			{/snippet}
		</EmptyState>
	{:else}
		<div class="overflow-x-auto rounded-xl border border-arch-line bg-arch-surface">
			<table class="w-full min-w-[560px] border-collapse text-left">
				<thead>
					<tr class="border-b border-arch-line">
						{#each ['Title', 'Status', 'Updated', 'Actions'] as heading (heading)}
							<th
								scope="col"
								class="px-[18px] py-3 font-mono text-[9px] font-normal tracking-[0.9px]
								       text-arch-muted uppercase {heading === 'Actions' ? 'sr-only' : ''}"
							>
								{heading}
							</th>
						{/each}
					</tr>
				</thead>
				<tbody>
					{#each posts as post (post.id)}
						<tr class="border-b border-arch-line last:border-b-0">
							<td class="px-[18px] py-3 text-[13px] font-medium text-arch-headline">
								{post.title}
							</td>
							<td class="px-[18px] py-3">
								<StatusPill tone="dormant" label="Archived" />
							</td>
							<td class="px-[18px] py-3 text-[12px] text-arch-muted">
								<time datetime={post.updated_at}>{updatedLabel(post.updated_at)}</time>
							</td>
							<td class="px-[18px] py-3">
								<div class="flex justify-end gap-2">
									<!-- The visible label is the verb; the accessible name says
									     which post, because a list of identical "Restore" buttons
									     is a list nobody can tell apart. -->
									<button
										type="button"
										aria-label="Restore {post.title}"
										disabled={restoring === post.id}
										aria-busy={restoring === post.id}
										onclick={() => restore(post)}
										class="inline-flex min-h-9 items-center rounded-lg border
										       border-arch-line-control px-3 text-[12.5px] font-semibold
										       text-arch-headline disabled:opacity-55"
									>
										Restore
									</button>
									<button
										type="button"
										aria-label="Delete {post.title} forever"
										onclick={() => {
											failure = undefined;
											purging = post;
										}}
										class="inline-flex min-h-9 items-center rounded-lg px-3 text-[12.5px]
										       font-semibold text-st-danger"
									>
										Delete forever
									</button>
								</div>
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>

		<div class="flex items-center justify-between text-[12px] text-arch-muted">
			<p>{posts.length} of {total} archived posts</p>
			{#if lastPage > 1}
				<div class="flex items-center gap-1">
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
		</div>
	{/if}
</div>

<ConfirmDialog
	open={purging !== null}
	title="Delete this post forever?"
	consequence="It cannot be restored. Anyone with its link will find nothing there."
	match={purging?.title ?? ''}
	confirmLabel="Delete forever"
	{working}
	failure={purging ? failure : undefined}
	onconfirm={purge}
	oncancel={() => (purging = null)}
/>

{#if toast}
	<div class="fixed inset-x-4 bottom-4 z-10 md:right-6 md:left-auto md:w-[380px]">
		<Toast message={toast} onclose={() => (toast = undefined)} />
	</div>
{/if}
