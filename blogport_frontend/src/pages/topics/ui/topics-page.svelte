<script lang="ts">
	import { Button, EmptyState, Field, InlineAlert } from '$lib/shared/ui';
	import { retireQuestion, type Topic, type TopicUsage } from '$lib/entities/topic';
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
	 * **No usage column, and no sort by usage.** §02 asks for both, from a line
	 * written before the usage endpoint existed — "assembled client-side from the
	 * post and project topic endpoints". One call per row is the same N+1 that
	 * §02 itself refused for the posts table's Topics column, and it refused it
	 * in the stronger case. Filed rather than built.
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

	let renaming = $state<string | null>(null);
	let draft = $state('');
	let retiring = $state<string | null>(null);
	let counted = $state<TopicUsage | null>(null);
	let counting = $state(false);
	let working = $state(false);
	let failure = $state<string | undefined>();
	let failureKind = $state<HandlingClass>('notOurs');

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
	<h1 class="font-display text-[19px] font-bold text-arch-headline">Topics</h1>

	<p class="text-[11.5px] text-arch-muted">
		One vocabulary across posts and projects. Renaming a topic keeps it on everything already tagged
		with it.
	</p>

	<InlineAlert message={failure} kind={failureKind} />

	{#if topics.length}
		<ul class="flex list-none flex-col gap-2.5 p-0">
			{#each topics as topic (topic.id)}
				<li class="rounded-xl border border-arch-line bg-arch-surface px-4 py-3.5">
					{#if renaming === topic.id}
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
					{:else}
						<div class="flex flex-wrap items-center justify-between gap-3">
							<div class="flex min-w-0 flex-col gap-0.5">
								<span class="text-[13px] font-medium text-arch-headline">{topic.title}</span>
								{#if topic.description}
									<span class="text-[11.5px] text-arch-muted">{topic.description}</span>
								{/if}
							</div>

							<!-- Raw buttons rather than the kit's: every row has a Rename
							     and a Retire, so the visible word alone names three
							     controls the same thing. The accessible name carries the
							     topic; the label stays as drawn. -->
							<div class="flex shrink-0 items-center gap-1.5">
								<button
									type="button"
									aria-label="Rename {topic.title}"
									onclick={() => openRename(topic)}
									class="rounded-md px-2.5 py-1.5 text-[12px] text-arch-muted
									       hover:bg-arch-surface-2 hover:text-arch-headline"
								>
									Rename
								</button>
								<button
									type="button"
									aria-label="Retire {topic.title}"
									onclick={() => askRetire(topic)}
									class="rounded-md px-2.5 py-1.5 text-[12px] text-arch-muted
									       hover:bg-arch-surface-2 hover:text-arch-headline"
								>
									Retire
								</button>
							</div>
						</div>

						{#if retiring === topic.id}
							<!-- A soft delete, asked plainly. The counted sentence sits where
							     the action is rather than behind a modal. -->
							<div class="mt-3 flex flex-col gap-2.5 rounded-lg bg-arch-surface-2 px-3 py-2.5">
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
						{/if}
					{/if}
				</li>
			{/each}
		</ul>
	{:else}
		<EmptyState
			title="No topics yet."
			message="Topics are the words you file posts and projects under, shared across both."
		/>
	{/if}
</div>
