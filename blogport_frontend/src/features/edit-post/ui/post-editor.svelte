<script lang="ts">
	import { untrack } from 'svelte';
	import { Field, InlineAlert, SaveIndicator, Toast } from '$lib/shared/ui';
	import type { HandlingClass } from '$lib/shared/lib/error-class';
	import { SLUG_MAX, slugError, slugFrom } from '$lib/shared/lib/slug';
	import { TITLE_COUNTER_FROM, TITLE_MAX, publicPostPath, titleError } from '$lib/entities/post';
	import { createAutosave } from '../model/autosave.svelte';
	import PublishControl from './publish-control.svelte';
	import { patchPost, type EditField, type PostChanges } from '../api/update-post';

	/**
	 * The editor — J4 steps two and three.
	 *
	 * Write, and it saves itself: PATCH on a two-second idle debounce, with the
	 * status line the only thing that reports save state. Only the keys that
	 * moved are sent, because PATCH changes what is present and the editor must
	 * never send an object it did not load first.
	 *
	 * Topics and the cover image are the steps after this one and are not here
	 * yet.
	 *
	 * Design: Screen / Post editor.
	 */
	type Post = {
		id: string;
		title: string;
		slug: string;
		content: string;
		published_at?: string | null;
	};

	let {
		post,
		/** Whose post it is. The public address is built from it. */
		username,
		/** Injected by the spec; the browser's own otherwise. */
		fetchFn = undefined
	}: {
		post: Post;
		username: string;
		fetchFn?: typeof globalThis.fetch;
	} = $props();

	// `untrack` because these are seeds, not bindings: the editor owns the text
	// from the moment it opens, and a loader that re-ran would otherwise throw
	// away whatever had been typed since.
	let title = $state(untrack(() => post.title));
	let slug = $state(untrack(() => post.slug));
	let content = $state(untrack(() => post.content));

	/** What the server has. The diff against it is what gets sent. */
	let saved = $state(
		untrack(() => ({ title: post.title, slug: post.slug, content: post.content }))
	);

	let titleProblem = $state<string | undefined>();
	let slugProblem = $state<string | undefined>();
	let failure = $state<string | undefined>();
	let failureKind = $state<HandlingClass>('notOurs');

	/**
	 * Owned here rather than read from the prop: publishing changes it without
	 * the loader running again, and the header has to follow immediately.
	 */
	let publishedAt = $state(untrack(() => post.published_at ?? null));
	let publishing = $state(false);
	let toast = $state<string | undefined>();

	const published = $derived(Boolean(publishedAt));
	const publicPath = $derived(publicPostPath(username, slug));

	/**
	 * Whether the address still looks like the title's.
	 *
	 * A draft's address follows the title, but only while nobody has written
	 * their own — and only while it is a draft. Once published, editing it
	 * breaks a live URL, so nothing may edit it by accident.
	 */
	let slugIsTheirs = $state(false);

	const under: Record<EditField, (message: string) => void> = {
		title: (message) => (titleProblem = message),
		slug: (message) => (slugProblem = message),
		// There is no separate error slot under the body: the server's only
		// content rule is "not empty", and the editor cannot reach that state
		// without the person deleting everything, which the alert covers.
		content: (message) => ((failure = message), (failureKind = 'field'))
	};

	function changes(): PostChanges {
		const moved: PostChanges = {};

		if (title !== saved.title) moved.title = title;
		if (slug !== saved.slug) moved.slug = slug;
		if (content !== saved.content) moved.content = content;

		return moved;
	}

	const autosave = createAutosave({
		async save() {
			const moved = changes();
			// Nothing left to send: something was typed and typed back again.
			if (Object.keys(moved).length === 0) return true;

			// The values as they went out. A save that lands is only proof about
			// these, not about whatever has been typed since.
			const sent = { title, slug, content };

			const result = await patchPost(post.id, moved, fetchFn);

			if (result.ok) {
				saved = sent;
				failure = undefined;
				return true;
			}

			if (result.field) {
				// A refused field is not something a retry will fix, so it is
				// reported where it belongs and the loop is told the save is done.
				under[result.field](result.message);
				saved = sent;
				return true;
			}

			failure = result.message;
			failureKind = result.kind;
			return false;
		}
	});

	$effect(() => () => autosave.destroy());

	function typedTitle() {
		if (titleProblem) titleProblem = titleError(title);

		if (!published && !slugIsTheirs) {
			slug = slugFrom(title);
			slugProblem = undefined;
		}

		autosave.edited();
	}

	function typedSlug() {
		slugIsTheirs = true;
		if (slugProblem) slugProblem = slugError(slug);
		autosave.edited();
	}

	/**
	 * `null` means now, and the timestamp is stamped here rather than in the
	 * control — so it is the moment the request went out and not the moment a
	 * component decided to offer the button.
	 */
	async function setPublished(at: string | null) {
		publishing = true;
		failure = undefined;

		// Saved first, so a post never goes live carrying words the server has
		// not taken.
		await autosave.flush();

		const result = await patchPost(post.id, { published_at: at }, fetchFn);

		publishing = false;

		if (!result.ok) {
			failure = result.message;
			failureKind = result.kind;
			return;
		}

		publishedAt = at;
		toast = at ? 'Published.' : 'Back to a draft.';
	}

	const titleCount = $derived([...title].length);
	const showCounter = $derived(titleCount >= TITLE_COUNTER_FROM);
</script>

<!-- eslint-disable svelte/no-navigation-without-resolve --
	Both links below go to the post's public address, which is a reader surface
	in the map and not built yet — `resolve()` only takes a route id that
	exists. Swap them in when /[username]/blog/[slug] lands. -->
<div class="flex max-w-[720px] flex-col gap-[18px] md:gap-5">
	<div class="flex items-center justify-between gap-4">
		<h1 class="sr-only">Edit post</h1>
		<!-- The only place that reports save state, in a slot that does not move. -->
		<SaveIndicator state={autosave.state} savedAt={autosave.savedAt} />

		{#if published}
			<a
				href={publicPath}
				class="text-[12px] font-semibold text-arch-accent-ink underline-offset-4 hover:underline"
			>
				View post
			</a>
		{/if}
	</div>

	<Field
		id="editor-title"
		label="Title"
		name="title"
		required
		bind:value={title}
		error={titleProblem}
		oninput={typedTitle}
		onblur={() => (titleProblem = titleError(title))}
	/>

	{#if showCounter}
		<p class="-mt-3 text-[11.5px] {titleCount > TITLE_MAX ? 'text-st-danger' : 'text-arch-muted'}">
			{titleCount} / {TITLE_MAX}
		</p>
	{/if}

	{#if published}
		<!-- Behind a disclosure, with the consequence stated: the post is live at
		     this address and changing it breaks every link to it. -->
		<details class="rounded-lg border border-arch-line bg-arch-surface p-3.5">
			<summary class="cursor-pointer text-[12.5px] font-semibold text-arch-headline">
				Change address
			</summary>
			<p class="mt-2 text-[12px] text-arch-muted">
				This post is live. Changing its address breaks the link anyone already has.
			</p>
			<div class="mt-3">
				<Field
					id="editor-slug"
					label="Web address"
					name="slug"
					maxlength={SLUG_MAX}
					required
					bind:value={slug}
					error={slugProblem}
					oninput={typedSlug}
					onblur={() => (slugProblem = slugError(slug))}
				/>
			</div>
		</details>
	{:else}
		<Field
			id="editor-slug"
			label="Web address"
			name="slug"
			maxlength={SLUG_MAX}
			help="This becomes the post's public address."
			required
			bind:value={slug}
			error={slugProblem}
			oninput={typedSlug}
			onblur={() => (slugProblem = slugError(slug))}
		/>
	{/if}

	<div class="flex flex-col gap-1.5">
		<label for="editor-content" class="text-[12.5px] font-medium text-arch-headline">Post</label>
		<!-- Not a Field: that component is a single-line input by specification. -->
		<textarea
			id="editor-content"
			name="content"
			rows="18"
			bind:value={content}
			oninput={() => autosave.edited()}
			class="w-full rounded-lg border border-arch-line-control bg-arch-surface px-3 py-2.5
			       font-mono text-[13px] text-arch-headline placeholder:text-arch-muted"
		></textarea>
	</div>

	<InlineAlert message={failure} kind={failureKind} />

	<PublishControl
		{publishedAt}
		busy={publishing}
		onpublish={(at) => setPublished(at ?? new Date().toISOString())}
		onunpublish={() => setPublished(null)}
	/>

	{#if toast}
		<div class="fixed inset-x-4 bottom-4 z-10 md:right-6 md:left-auto md:w-[380px]">
			<Toast message={toast} onclose={() => (toast = undefined)}>
				{#snippet action()}
					<a
						href={publicPath}
						class="text-[12.5px] font-semibold text-arch-accent-ink underline-offset-4
						       hover:underline"
					>
						View post
					</a>
				{/snippet}
			</Toast>
		</div>
	{/if}
</div>
