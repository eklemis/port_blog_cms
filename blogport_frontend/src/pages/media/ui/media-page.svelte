<script lang="ts">
	import { Button, EmptyState, Field, InlineAlert, StatusPill } from '$lib/shared/ui';
	import {
		archiveMedia,
		patchMedia,
		pillFor,
		purgeMedia,
		restoreMedia,
		tileStatus,
		type MediaRole,
		type MediaState
	} from '$lib/entities/media';
	import type { HandlingClass } from '$lib/shared/lib/error-class';

	/**
	 * `/studio/media` — Screen / Media library 69:242.
	 *
	 * **Scoped by target, because the listing is.** §01 refuses a combined view —
	 * "A combined library means four parallel calls merged client-side with no
	 * paging — scope it as a per-target picker instead" — and §02's route map
	 * scopes the route the same way. The frame draws an "All" tab and annotates
	 * it as four merged requests; that contradiction is with the designer, so the
	 * tab is not built while the other four are.
	 *
	 * **Archived tiles sit among the live ones**, as the frame draws them.
	 * `?include_deleted=true` returns both and `deleted_at` tells them apart —
	 * shipped because "archived media could already be restored and purged, but
	 * not *found*: the listing excluded it and the row carried no sign it
	 * existed, so a Restore control had nothing to act on."
	 *
	 * An archived tile is not nagged about its alt text. It is on nothing;
	 * describing it better is not the job in front of whoever is looking at it.
	 * Purge asks harder than Archive did, because it is the rung that stays.
	 *
	 * **Retry on a failed tile is not built** for a smaller reason: retrying
	 * means sending the bytes again, and after a reload the browser no longer has
	 * the file. Archive is offered instead, which is the honest half.
	 *
	 * What this screen is actually for: it is the only place a whole collection
	 * is visible at once, so it is the only place missing alt text can be seen as
	 * a set rather than one image at a time.
	 */
	type Item = {
		media_id: string;
		original_filename: string;
		role: MediaRole;
		status: MediaState;
		alt_text: string;
		/** A signed read URL, resolved with the page. Short-lived by design. */
		src?: string | null;
		/** When it was archived, or `null` while it is live. */
		deleted_at?: string | null;
	};

	let {
		items,
		scope,
		archived = false,
		onchanged = () => {},
		onquery,
		fetchFn = undefined
	}: {
		items: Item[];
		/** Whether archived tiles were asked for. */
		archived?: boolean;
		/**
		 * Which attachment target is being shown. Named `scope` rather than
		 * `target` because `target` is one of Svelte's own mount options, and a
		 * prop by that name is read as one.
		 */
		scope: string;
		onchanged?: () => void;
		/** A different target or archive setting was chosen; the caller reloads. */
		onquery: (target: string, archived: boolean) => void;
		fetchFn?: typeof globalThis.fetch;
	} = $props();

	/**
	 * The frame's tabs, minus "All". `avatar` is the label the frame uses for the
	 * `user` target, which is what an avatar hangs off.
	 */
	const TARGETS = [
		{ id: 'blog_post', label: 'Posts', noun: 'posts' },
		{ id: 'project', label: 'Projects', noun: 'projects' },
		{ id: 'resume', label: 'Résumés', noun: 'résumés' },
		{ id: 'user', label: 'Avatar', noun: 'profile' }
	];

	const noun = $derived(TARGETS.find((entry) => entry.id === scope)?.noun ?? 'posts');

	let editing = $state<string | null>(null);
	let draft = $state('');
	let archiving = $state<string | null>(null);
	let purging = $state<string | null>(null);
	let working = $state(false);
	let failure = $state<string | undefined>();
	let failureKind = $state<HandlingClass>('notOurs');

	function openEdit(item: Item) {
		archiving = null;
		editing = item.media_id;
		draft = item.alt_text;
		failure = undefined;
	}

	async function saveAlt(item: Item) {
		if (!draft.trim() || working) return;

		working = true;
		const result = await patchMedia(item.media_id, { alt_text: draft.trim() }, fetchFn);
		working = false;

		if (!result.ok) {
			failure = result.message;
			failureKind = result.kind;
			return;
		}

		editing = null;
		onchanged();
	}

	async function restore(item: Item) {
		if (working) return;

		working = true;
		const result = await restoreMedia(item.media_id, fetchFn);
		working = false;

		if (!result.ok) {
			failure = result.message;
			failureKind = result.kind;
			return;
		}

		onchanged();
	}

	async function confirmPurge(item: Item) {
		if (working) return;

		working = true;
		const result = await purgeMedia(item.media_id, fetchFn);
		working = false;

		if (!result.ok) {
			failure = result.message;
			failureKind = result.kind;
			return;
		}

		purging = null;
		onchanged();
	}

	async function confirmArchive(item: Item) {
		if (working) return;

		working = true;
		const result = await archiveMedia(item.media_id, fetchFn);
		working = false;

		if (!result.ok) {
			failure = result.message;
			failureKind = result.kind;
			return;
		}

		archiving = null;
		onchanged();
	}
</script>

<div class="flex flex-col gap-4">
	<h1 class="font-display text-[19px] font-bold text-arch-headline">Media</h1>

	<div class="flex flex-wrap items-center gap-2">
		{#each TARGETS as entry (entry.id)}
			{@const current = entry.id === scope}
			<button
				type="button"
				aria-current={current ? 'true' : undefined}
				onclick={() => onquery(entry.id, archived)}
				class="rounded-full border px-3 py-1.5 text-[12px]
				       {current
					? 'border-arch-accent-ink font-semibold text-arch-accent-ink'
					: 'border-arch-line text-arch-muted hover:text-arch-headline'}"
			>
				{entry.label}
			</button>
		{/each}
	</div>

	<div class="flex flex-wrap items-center justify-between gap-3">
		<!-- The frame says this out loud, and it is the reason there is no "All". -->
		<p class="text-[11.5px] text-arch-muted">
			Listing is by target, so this is one collection at a time.
		</p>

		<label class="flex items-center gap-2 text-[11.5px] text-arch-muted">
			<input
				type="checkbox"
				checked={archived}
				onchange={(event) => onquery(scope, event.currentTarget.checked)}
				class="size-3.5 accent-arch-accent"
			/>
			Showing archived
		</label>
	</div>

	<InlineAlert message={failure} kind={failureKind} />

	{#if items.length}
		<ul class="grid list-none grid-cols-1 gap-4 p-0 sm:grid-cols-2 lg:grid-cols-4">
			{#each items as item (item.media_id)}
				{@const status = tileStatus(item)}
				{@const pill = pillFor(item.status)}
				{@const gone = Boolean(item.deleted_at)}
				<li
					class="flex flex-col overflow-hidden rounded-xl border bg-arch-surface
					       {item.status === 'failed' ? 'border-st-danger' : 'border-arch-line'}"
				>
					<div class="flex h-[104px] items-center justify-center bg-arch-surface-2">
						{#if gone}
							<StatusPill tone="dormant" label="Archived" />
						{:else if item.status === 'ready' && item.src}
							<img src={item.src} alt={item.alt_text} class="size-full object-cover" />
						{:else if pill}
							<StatusPill tone={pill.tone} label={pill.label} />
						{/if}
					</div>

					<div class="flex flex-col gap-1 px-3.5 py-3">
						<p class="truncate font-mono text-[11.5px] text-arch-headline">
							{item.original_filename}
						</p>
						<p
							class="text-[11px] {!gone && status.prompt
								? 'text-arch-accent-ink'
								: 'text-arch-muted'}"
						>
							{gone ? 'Restore or purge' : status.text}
						</p>

						{#if editing === item.media_id}
							<div class="mt-1.5 flex flex-col gap-2">
								<Field id="alt-{item.media_id}" label="Alt text" bind:value={draft} />
								<div class="flex gap-2">
									<Button
										size="compact"
										label="Save"
										disabled={!draft.trim()}
										loading={working}
										onclick={() => saveAlt(item)}
									/>
									<Button
										kind="ghost"
										size="compact"
										label="Cancel"
										onclick={() => (editing = null)}
									/>
								</div>
							</div>
						{:else if archiving === item.media_id}
							<div class="mt-1.5 flex flex-col gap-2">
								<p class="text-[11px] text-arch-headline" role="status">
									Archive this image? It comes off anything using it.
								</p>
								<div class="flex gap-2">
									<Button
										kind="danger"
										size="compact"
										label="Archive image"
										loading={working}
										onclick={() => confirmArchive(item)}
									/>
									<Button
										kind="ghost"
										size="compact"
										label="Cancel"
										onclick={() => (archiving = null)}
									/>
								</div>
							</div>
						{:else if purging === item.media_id}
							<div class="mt-1.5 flex flex-col gap-2">
								<p class="text-[11px] text-arch-headline" role="status">
									Purge this image? This one does not come back.
								</p>
								<div class="flex gap-2">
									<Button
										kind="danger"
										size="compact"
										label="Purge for good"
										loading={working}
										onclick={() => confirmPurge(item)}
									/>
									<Button
										kind="ghost"
										size="compact"
										label="Cancel"
										onclick={() => (purging = null)}
									/>
								</div>
							</div>
						{:else if gone}
							<!-- Two rungs left, and no nagging about alt text: it is on
							     nothing, so describing it better is not the job. -->
							<div class="mt-0.5 flex items-center gap-3">
								<button
									type="button"
									aria-label="Restore {item.original_filename}"
									onclick={() => restore(item)}
									class="text-[11px] text-arch-accent-ink hover:underline"
								>
									Restore
								</button>
								<button
									type="button"
									aria-label="Purge {item.original_filename}"
									onclick={() => (purging = item.media_id)}
									class="text-[11px] text-st-danger hover:underline"
								>
									Purge
								</button>
							</div>
						{:else}
							<div class="mt-0.5 flex items-center gap-3">
								<button
									type="button"
									aria-label="Edit alt text for {item.original_filename}"
									onclick={() => openEdit(item)}
									class="text-[11px] text-arch-accent-ink hover:underline"
								>
									Edit alt text
								</button>
								<button
									type="button"
									aria-label="Archive {item.original_filename}"
									onclick={() => {
										editing = null;
										archiving = item.media_id;
									}}
									class="text-[11px] text-arch-muted hover:text-arch-headline"
								>
									Archive
								</button>
							</div>
						{/if}
					</div>
				</li>
			{/each}
		</ul>
	{:else}
		<EmptyState
			title="No images on your {noun} yet."
			message="Images arrive here when you add them to a post or a project — this screen is where you keep them honest."
		/>
	{/if}
</div>
