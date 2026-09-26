<script lang="ts">
	import { Plus } from '@lucide/svelte';
	import { Button, EmptyState, Field, InlineAlert } from '$lib/shared/ui';
	import {
		createTopic,
		retireQuestion,
		usageLine,
		type Topic,
		type TopicUsage
	} from '$lib/entities/topic';
	import { renameTopic, retireTopic, topicUsage } from '$lib/features/manage-topics';
	import type { HandlingClass } from '$lib/shared/lib/error-class';

	/**
	 * `/studio/topics` — the shared vocabulary. Screen / Topics 70:382.
	 *
	 * **Renaming is here, and §02's journey says it should not be.** That journey
	 * reads "Retire, don't rename — only soft delete exists… No update endpoint.
	 * A typo is unfixable", and prescribes create-retag-retire by hand.
	 * `PATCH /api/topics/{id}` has since shipped and says what it replaced: "The
	 * topic keeps its id, so everything tagged with it follows the new name
	 * automatically — nothing needs retagging." Raised with the designer.
	 *
	 * **Retiring counts first.** §02: "GET /api/topics/{id}/usage returns the real
	 * numbers, so the dialog can say 'Retire «Rust»? It's on 6 posts and 2
	 * projects.' Never drop a topic off eight pages silently." The count is asked
	 * for when Retire is pressed, which is the moment it is needed.
	 *
	 * **The USED ON column is a render, not a fetch.** It was parked while
	 * `…/usage` was the only source — one topic per request, on an unpaged list,
	 * is the N+1 §02 refused for the posts table in the weaker case. The counts
	 * now come back on the listing itself, computed in the statement that
	 * already runs.
	 *
	 * The column and the retire confirmation share one formatter deliberately.
	 * Both counts exclude soft-deleted rows, and two call sites doing their own
	 * arithmetic is how a row and a dialog agree right up until somebody
	 * archives a post.
	 *
	 * Retiring is a soft delete and reads as one: a plain confirmation in the
	 * row, not the type-to-confirm dialog, which is for things that do not come
	 * back.
	 */
	let {
		topics,
		onchanged = () => {},
		fetchFn = undefined
	}: {
		topics: Topic[];
		/** The vocabulary changed; the caller reloads it. */
		onchanged?: () => void;
		fetchFn?: typeof globalThis.fetch;
	} = $props();

	let creating = $state(false);
	let newTitle = $state('');
	let newDescription = $state('');
	let renaming = $state<string | null>(null);
	let draft = $state('');
	let retiring = $state<string | null>(null);
	let counted = $state<TopicUsage | null>(null);
	let counting = $state(false);
	let working = $state(false);
	let failure = $state<string | undefined>();
	let failureKind = $state<HandlingClass>('notOurs');

	async function create() {
		if (!newTitle.trim() || working) return;

		working = true;
		const result = await createTopic(newTitle.trim(), fetchFn, newDescription);
		working = false;

		if (!result.ok) {
			failure = result.message;
			failureKind = result.kind;
			return;
		}

		creating = false;
		newTitle = '';
		newDescription = '';
		onchanged();
	}

	function openRename(topic: Topic) {
		retiring = null;
		renaming = topic.id;
		draft = topic.title;
		failure = undefined;
	}

	async function saveName(topic: Topic) {
		if (!draft.trim() || working) return;

		working = true;
		const result = await renameTopic(topic.id, draft.trim(), fetchFn);
		working = false;

		if (!result.ok) {
			failure = result.message;
			failureKind = result.kind;
			return;
		}

		renaming = null;
		onchanged();
	}

	async function askRetire(topic: Topic) {
		renaming = null;
		retiring = topic.id;
		counted = null;
		counting = true;
		failure = undefined;

		counted = await topicUsage(topic.id, fetchFn);
		counting = false;
	}

	async function confirmRetire(topic: Topic) {
		if (working) return;

		working = true;
		const result = await retireTopic(topic.id, fetchFn);
		working = false;

		if (!result.ok) {
			failure = result.message;
			failureKind = result.kind;
			return;
		}

		retiring = null;
		onchanged();
	}
</script>

<div class="flex flex-col gap-4">
	<div class="flex flex-wrap items-center justify-between gap-3">
		<h1 class="font-display text-[19px] font-bold text-arch-headline">Topics</h1>
		<Button
			label="New topic"
			onclick={() => {
				renaming = null;
				retiring = null;
				creating = !creating;
			}}
		>
			{#snippet icon()}<Plus size={15} aria-hidden="true" />{/snippet}
		</Button>
	</div>

	<p class="text-[11.5px] text-arch-muted">
		One vocabulary across posts and projects. Most topics are born inside an editor — this screen is
		for tidying.
	</p>

	<InlineAlert message={failure} kind={failureKind} />

	{#if creating}
		<!-- §02: "title and description in the same popover — a taxonomy of bare
		     words stops being useful at about fifteen entries." -->
		<div class="flex flex-col gap-3 rounded-xl border border-arch-line bg-arch-surface p-5">
			<Field id="new-topic-title" label="Title" bind:value={newTitle} />
			<Field
				id="new-topic-description"
				label="Description"
				bind:value={newDescription}
				help="What belongs under this word. Optional, and worth writing."
			/>
			<div class="flex gap-2">
				<Button
					label="Create topic"
					disabled={!newTitle.trim()}
					loading={working}
					onclick={create}
				/>
				<Button kind="ghost" label="Cancel" onclick={() => (creating = false)} />
			</div>
		</div>
	{/if}

	{#if topics.length}
		<div class="overflow-x-auto rounded-xl border border-arch-line bg-arch-surface">
			<table class="w-full border-collapse text-left">
				<thead>
					<tr class="border-b border-arch-line">
						<th
							scope="col"
							class="px-5 py-3 font-mono text-[9px] font-normal tracking-[0.9px] text-arch-muted
							       uppercase"
						>
							Topic
						</th>
						<th
							scope="col"
							class="px-5 py-3 font-mono text-[9px] font-normal tracking-[0.9px] text-arch-muted
							       uppercase"
						>
							Description
						</th>
						<th
							scope="col"
							class="px-5 py-3 font-mono text-[9px] font-normal tracking-[0.9px] text-arch-muted
							       uppercase"
						>
							Used on
						</th>
						<th scope="col" class="px-5 py-3"><span class="sr-only">Actions</span></th>
					</tr>
				</thead>
				<tbody>
					{#each topics as topic (topic.id)}
						<tr class="border-b border-arch-line last:border-0">
							{#if renaming === topic.id}
								<td colspan="4" class="px-5 py-3.5">
									<div class="flex flex-col gap-2.5">
										<Field id="topic-{topic.id}-title" label="Title" bind:value={draft} />
										<div class="flex gap-2">
											<Button
												label="Save"
												disabled={!draft.trim()}
												loading={working}
												onclick={() => saveName(topic)}
											/>
											<Button kind="ghost" label="Cancel" onclick={() => (renaming = null)} />
										</div>
									</div>
								</td>
							{:else}
								<td class="px-5 py-3.5 text-[13px] font-medium text-arch-headline">
									{topic.title}
								</td>
								<td class="px-5 py-3.5 text-[12.5px] text-arch-muted">
									{topic.description ?? ''}
								</td>
								<td class="px-5 py-3.5 font-mono text-[11.5px] text-arch-muted">
									{usageLine({
										posts: topic.post_count ?? 0,
										projects: topic.project_count ?? 0
									})}
								</td>
								<td class="px-5 py-3.5">
									<div class="flex items-center justify-end gap-1.5">
										<button
											type="button"
											aria-label="Rename {topic.title}"
											onclick={() => openRename(topic)}
											class="rounded-md px-2.5 py-1.5 text-[12px] text-arch-accent-ink
											       hover:underline"
										>
											Rename
										</button>
										<button
											type="button"
											aria-label="Retire {topic.title}"
											onclick={() => askRetire(topic)}
											class="rounded-md px-2.5 py-1.5 text-[12px] text-arch-muted
											       hover:text-arch-headline"
										>
											Retire
										</button>
									</div>
								</td>
							{/if}
						</tr>

						{#if retiring === topic.id}
							<tr class="border-b border-arch-line last:border-0">
								<td colspan="4" class="px-5 pb-3.5">
									<!-- A soft delete, asked plainly. The counted sentence sits
									     where the action is rather than behind a modal. -->
									<div class="flex flex-col gap-2.5 rounded-lg bg-arch-surface-2 px-3 py-2.5">
										<p class="text-[12.5px] text-arch-headline" role="status">
											{counting ? `Retire «${topic.title}»?` : retireQuestion(topic.title, counted)}
										</p>
										<div class="flex gap-2">
											<Button
												kind="danger"
												size="compact"
												label="Retire topic"
												loading={working}
												onclick={() => confirmRetire(topic)}
											/>
											<Button
												kind="ghost"
												size="compact"
												label="Cancel"
												onclick={() => (retiring = null)}
											/>
										</div>
									</div>
								</td>
							</tr>
						{/if}
					{/each}
				</tbody>
			</table>
		</div>
	{:else}
		<EmptyState
			title="No topics yet."
			message="Topics are the words you file posts and projects under, shared across both."
		/>
	{/if}
</div>
