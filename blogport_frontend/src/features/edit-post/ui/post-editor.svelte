<script lang="ts">
	import { untrack } from 'svelte';
	import { ChevronLeft } from '@lucide/svelte';
	import { InlineAlert, SaveIndicator, StatusPill, Toast } from '$lib/shared/ui';
	import { CONSOLE_ROUTES } from '$lib/shared/config/routes';
	import type { HandlingClass } from '$lib/shared/lib/error-class';
	import { SLUG_MAX, slugError, slugFrom } from '$lib/shared/lib/slug';
	import {
		TITLE_COUNTER_FROM,
		TITLE_MAX,
		postStatus,
		publicPostPath,
		titleError
	} from '$lib/entities/post';
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
	 * Design: Screen / Post editor 32:934 · Mobile / Post editor 73:236. A top bar
	 * with the status pill, the save state and the publish buttons; the post in
	 * one card, its title and address inline; a rail beside it. On a phone the
	 * editor draws its own bar — back, pill, Publish — and the shell draws none.
	 *
	 * The rail's Assist and Cover image cards and the topic picker's "+ Add" are
	 * drawn and not built: the companion rail, the upload flow and TopicPicker
	 * are their own slices. The rail lists the post's topics as they are.
	 */
	type Post = {
		id: string;
		title: string;
		slug: string;
		content: string;
		published_at?: string | null;
		topics?: { id: string; title: string }[];
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

	const status = $derived(postStatus(publishedAt));
	/** The fixed part of the public address, muted in the frame. */
	const addressPrefix = $derived(`/${username}/blog/`);

	const titleCount = $derived([...title].length);
	const showCounter = $derived(titleCount >= TITLE_COUNTER_FROM);
</script>

<!-- eslint-disable svelte/no-navigation-without-resolve --
	Both links below go to the post's public address, which is a reader surface
	in the map and not built yet — `resolve()` only takes a route id that
	exists. Swap them in when /[username]/blog/[slug] lands. -->
<div class="flex flex-col gap-4">
	<h1 class="sr-only">Edit post</h1>

	<!-- The top bar: status and save state on the left, the one decision on the right. -->
	<section aria-label="Post status" class="flex min-h-[38px] items-center justify-between gap-3">
		<div class="flex items-center gap-3">
			<a
				href={CONSOLE_ROUTES.posts}
				aria-label="Back to Posts"
				class="-ml-2 flex size-8 items-center justify-center text-arch-headline md:hidden"
			>
				<ChevronLeft size={19} aria-hidden="true" />
			</a>
			<StatusPill tone={status.tone} label={status.label} />
			<!-- The only place that reports save state. Mobile / Post editor draws it
			     under the body; it stays here at every width, because a second copy
			     would be a second live region announcing the same thing. -->
			<div class="font-mono">
				<SaveIndicator state={autosave.state} savedAt={autosave.savedAt} />
			</div>
		</div>

		<div class="flex items-center gap-2">
			{#if published}
				<a
					href={publicPath}
					class="px-3 text-[13px] font-semibold text-arch-muted hover:text-arch-headline max-md:hidden"
				>
					View post
				</a>
			{/if}
			<PublishControl
				{publishedAt}
				busy={publishing}
				onpublish={(at) => setPublished(at ?? new Date().toISOString())}
				onunpublish={() => setPublished(null)}
			/>
		</div>
	</section>

	{#if published}
		<p class="-mt-2 text-[11.5px] text-arch-muted">
			Live at {publicPath}. Unpublishing puts it back to a draft, and that address stops working.
		</p>
	{/if}

	<InlineAlert message={failure} kind={failureKind} />

	<div class="flex flex-col gap-4 lg:flex-row lg:items-start">
		<!-- The post itself: title, address, rule, body — one card. -->
		<div
			class="flex min-w-0 flex-1 flex-col gap-3 md:rounded-xl md:border md:border-arch-line
			       md:bg-arch-surface md:p-5"
		>
			<label for="editor-title" class="sr-only">Title</label>
			<input
				id="editor-title"
				name="title"
				bind:value={title}
				oninput={typedTitle}
				onblur={() => (titleProblem = titleError(title))}
				aria-invalid={titleProblem ? 'true' : undefined}
				aria-describedby={titleProblem ? 'editor-title-error' : undefined}
				class="w-full bg-transparent font-display text-[24px] font-extrabold text-arch-headline
				       focus:outline-none md:text-[26px]"
			/>
			{#if titleProblem}
				<p id="editor-title-error" class="-mt-2 text-[11px] text-st-danger">{titleProblem}</p>
			{/if}
			{#if showCounter}
				<p
					class="-mt-2 text-[11px] {titleCount > TITLE_MAX ? 'text-st-danger' : 'text-arch-muted'}"
				>
					{titleCount} / {TITLE_MAX}
				</p>
			{/if}

			<div class="flex flex-col gap-1">
				<div class="flex items-center font-mono text-[11px]">
					<span aria-hidden="true" class="text-arch-muted">{addressPrefix}</span>
					<label for="editor-slug" class="sr-only">Address</label>
					{#if published}
						<!-- Live: the address is read here and changed only through the
						     disclosure below, because changing it breaks every link. -->
						<span class="text-arch-accent-ink">{slug}</span>
					{:else}
						<input
							id="editor-slug"
							name="slug"
							maxlength={SLUG_MAX}
							bind:value={slug}
							oninput={typedSlug}
							onblur={() => (slugProblem = slugError(slug))}
							aria-invalid={slugProblem ? 'true' : undefined}
							aria-describedby={slugProblem ? 'editor-slug-error' : undefined}
							class="min-w-0 flex-1 bg-transparent text-arch-accent-ink focus:outline-none"
						/>
					{/if}
				</div>
				{#if published}
					<details>
						<summary class="cursor-pointer text-[11.5px] text-arch-muted">Change address</summary>
						<p class="mt-1.5 text-[11.5px] text-arch-muted">
							This post is live. Changing its address breaks the link anyone already has.
						</p>
						<input
							id="editor-slug"
							name="slug"
							maxlength={SLUG_MAX}
							bind:value={slug}
							oninput={typedSlug}
							onblur={() => (slugProblem = slugError(slug))}
							aria-invalid={slugProblem ? 'true' : undefined}
							aria-describedby={slugProblem ? 'editor-slug-error' : undefined}
							class="mt-2 w-full rounded-lg border border-arch-line-control bg-arch-surface px-3 py-2
							       font-mono text-[12px] text-arch-headline"
						/>
					</details>
				{/if}
				{#if slugProblem}
					<p id="editor-slug-error" class="text-[11px] text-st-danger">{slugProblem}</p>
				{/if}
			</div>

			<div class="h-px bg-arch-line" role="presentation"></div>

			<label for="editor-content" class="sr-only">Post</label>
			<textarea
				id="editor-content"
				name="content"
				rows="20"
				bind:value={content}
				oninput={() => autosave.edited()}
				placeholder="Write the post."
				class="min-h-[420px] w-full resize-none bg-transparent text-[13px] leading-[22px]
				       text-arch-headline placeholder:text-arch-muted focus:outline-none"
			></textarea>
		</div>

		<!-- The rail. Topics as the post has them; picking them is TopicPicker's. -->
		<div class="flex w-full flex-col gap-3.5 lg:w-[346px]">
			<section
				aria-label="Topics"
				class="flex flex-col gap-2.5 rounded-xl border border-arch-line bg-arch-surface px-4 py-[15px]"
			>
				<h2 class="font-mono text-[9px] font-normal tracking-[0.9px] text-arch-muted uppercase">
					Topics
				</h2>
				{#if post.topics?.length}
					<ul class="flex flex-wrap gap-[7px]">
						{#each post.topics as topic (topic.id)}
							<li
								class="rounded-full bg-arch-surface-2 px-2.5 py-[5px] text-[11px] text-arch-headline"
							>
								{topic.title}
							</li>
						{/each}
					</ul>
				{:else}
					<p class="text-[11.5px] text-arch-muted">No topics yet.</p>
				{/if}
			</section>
		</div>
	</div>

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
