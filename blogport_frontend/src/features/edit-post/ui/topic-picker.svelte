<script lang="ts">
	import { Plus, X } from '@lucide/svelte';
	import type { Topic } from '../api/topics';

	/**
	 * The editor rail's topics — the chips a post carries, and the "+ Add" that
	 * puts another on.
	 *
	 * §03 calls this a combobox: "Search + multi-select + inline create. Offers
	 * 'Create «Rust»' when nothing matches." It is built as a labelled search
	 * field over a list of buttons rather than as an ARIA combobox with
	 * `aria-activedescendant`: the behaviour the spec asks for is all here, and a
	 * half-built combobox announces itself as something it cannot do.
	 *
	 * Nothing here calls the API. Each chip is its own request (§03), and the
	 * editor owns which ones are in flight — a control that decided that for
	 * itself would be a control that could disagree with the post.
	 */
	let {
		attached,
		available,
		onattach = () => {},
		ondetach = () => {},
		oncreate = () => {}
	}: {
		/** The topics the post has now. */
		attached: Topic[];
		/** Every topic this author owns — `GET /api/topics` is their own only. */
		available: Topic[];
		onattach?: (topic: Topic) => void;
		ondetach?: (topic: Topic) => void;
		/** A topic that does not exist yet. Creating and attaching are two calls. */
		oncreate?: (title: string) => void;
	} = $props();

	let open = $state(false);
	let query = $state('');
	let trigger = $state<HTMLButtonElement>();
	let field = $state<HTMLInputElement>();

	// Focus lands in the search box when the picker opens. It was opened by a
	// deliberate press and typing is the only thing to do in it — done here
	// rather than with `autofocus`, which would also fire on page load.
	$effect(() => {
		if (open) field?.focus();
	});

	const listId = $props.id();

	const on = $derived(new Set(attached.map((topic) => topic.id)));

	/** Not already on the post, and matching what has been typed. */
	const offered = $derived(
		available
			.filter((topic) => !on.has(topic.id))
			.filter((topic) => topic.title.toLowerCase().includes(query.trim().toLowerCase()))
	);

	/**
	 * §03 offers to create "when nothing matches" — so a search that still has
	 * something to show does not also offer to make a near-duplicate of it.
	 * Typing "arch" against an existing "Architecture" offers Architecture.
	 *
	 * The exact-title check is separate because a topic already on the post is
	 * filtered out of `offered`, and typing its name must not offer to make it
	 * a second time.
	 */
	const creatable = $derived.by(() => {
		const wanted = query.trim();
		if (!wanted || offered.length) return null;

		const exists = available.some((topic) => topic.title.toLowerCase() === wanted.toLowerCase());
		return exists ? null : wanted;
	});

	function close({ toTrigger = false } = {}) {
		open = false;
		query = '';
		if (toTrigger) trigger?.focus();
	}
</script>

<svelte:window
	onkeydown={(event) => {
		if (open && event.key === 'Escape') close({ toTrigger: true });
	}}
/>

<section
	aria-label="Topics"
	class="flex flex-col gap-2.5 rounded-xl border border-arch-line bg-arch-surface px-4 py-[15px]"
>
	<h2 class="font-mono text-[9px] font-normal tracking-[0.9px] text-arch-muted uppercase">
		Topics
	</h2>

	<div class="flex flex-wrap items-center gap-[7px]">
		{#each attached as topic (topic.id)}
			<span
				class="flex items-center gap-1 rounded-full bg-arch-surface-2 py-[5px] pr-1.5 pl-2.5
				       text-[11px] text-arch-headline"
			>
				{topic.title}
				<button
					type="button"
					aria-label="Remove {topic.title}"
					onclick={() => ondetach(topic)}
					class="flex size-4 items-center justify-center rounded-full text-arch-muted
					       hover:text-arch-headline"
				>
					<X size={11} aria-hidden="true" />
				</button>
			</span>
		{/each}

		<button
			bind:this={trigger}
			type="button"
			aria-expanded={open}
			aria-controls={listId}
			onclick={() => (open ? close({ toTrigger: true }) : (open = true))}
			class="flex items-center gap-1 rounded-full border border-arch-line-control px-2.5 py-[5px]
			       text-[11px] text-arch-headline hover:border-arch-line-strong"
		>
			<Plus size={11} aria-hidden="true" />
			Add a topic
		</button>
	</div>

	{#if !attached.length}
		<p class="text-[11.5px] text-arch-muted">No topics yet.</p>
	{/if}

	{#if open}
		<div id={listId} class="flex flex-col gap-2 rounded-lg border border-arch-line p-2.5">
			<label class="flex flex-col gap-1 text-[11px] text-arch-muted">
				Search topics
				<input
					bind:this={field}
					type="text"
					bind:value={query}
					class="rounded-md border border-arch-line-control bg-arch-surface px-2 py-1.5
					       text-[12px] text-arch-headline"
				/>
			</label>

			{#if offered.length}
				<ul class="flex flex-col">
					{#each offered as topic (topic.id)}
						<li>
							<button
								type="button"
								onclick={() => onattach(topic)}
								class="w-full rounded px-2 py-1.5 text-left text-[12px] text-arch-headline
								       hover:bg-arch-surface-2"
							>
								{topic.title}
							</button>
						</li>
					{/each}
				</ul>
			{/if}

			{#if creatable}
				<!-- §03: "Offers 'Create «Rust»' when nothing matches." Creating and
				     attaching are two requests; the editor makes both. -->
				<button
					type="button"
					onclick={() => {
						oncreate(creatable);
						close({ toTrigger: true });
					}}
					class="rounded px-2 py-1.5 text-left text-[12px] font-semibold text-arch-accent-ink
					       hover:bg-arch-surface-2"
				>
					Create “{creatable}”
				</button>
			{:else if !offered.length}
				<p class="px-2 py-1.5 text-[11.5px] text-arch-muted">No topics left to add.</p>
			{/if}
		</div>
	{/if}
</section>
