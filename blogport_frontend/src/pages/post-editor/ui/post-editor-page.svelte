<script lang="ts">
	import { PostEditor } from '$lib/features/edit-post';
	import { NO_ACCESS } from '$lib/features/edit-post';
	import { archivePost } from '$lib/features/manage-archive';
	import { InlineAlert } from '$lib/shared/ui';
	import type { HandlingClass } from '$lib/shared/lib/error-class';
	import { Button, EmptyState } from '$lib/shared/ui';
	import { CONSOLE_ROUTES } from '$lib/shared/config/routes';
	import type { MediaState } from '$lib/entities/media';

	/**
	 * `/studio/posts/[id]`.
	 *
	 * Two shapes. The editor, and the page someone reaches by pasting a URL for
	 * a post that is not theirs — J4 asks for a plain sentence and a route back
	 * to the list, and never a bounce through login.
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

	let failure = $state<string | undefined>();
	let failureKind = $state<HandlingClass>('notOurs');

	let {
		post,
		username,
		onarchived = () => {},
		onpreview = () => {},
		/** The author's own topics, for the editor's rail picker. */
		availableTopics = [],
		/** This post's cover, and a signed URL for it when one is ready. */
		cover = null,
		coverSrc = null,
		/** The cover changed; the caller reloads rather than this guessing. */
		oncoverchange = () => {},
		fetchFn = undefined,
		denied = false
	}: {
		post: Post | null;
		/** Whose post it is. The public address is built from it. */
		username: string;
		/** The post is archived — the caller leaves for the list. */
		onarchived?: () => void;
		/** A preview link is ready, for the caller to open in a new tab. */
		onpreview?: (path: string) => void;
		availableTopics?: { id: string; title: string }[];
		cover?: { media_id: string; status: MediaState; alt_text: string } | null;
		coverSrc?: string | null;
		oncoverchange?: () => void;
		/** Injected by the spec; the browser's own otherwise. */
		fetchFn?: typeof globalThis.fetch;
		denied?: boolean;
	} = $props();
</script>

{#if denied || !post}
	<EmptyState
		title={NO_ACCESS}
		message="It may belong to someone else, or it may have been deleted."
	>
		{#snippet action()}
			<Button kind="secondary" label="Back to posts" href={CONSOLE_ROUTES.posts} />
		{/snippet}
	</EmptyState>
{:else}
	<InlineAlert message={failure} kind={failureKind} />
	<PostEditor
		{post}
		{username}
		{onpreview}
		{availableTopics}
		{cover}
		{coverSrc}
		{oncoverchange}
		onarchive={async () => {
			const result = await archivePost(post.id, fetchFn);

			if (!result.ok) {
				failure = result.message;
				failureKind = result.kind;
				return;
			}

			// The undo toast belongs to the list this leaves for: an editor for a
			// post that is no longer there has nothing left to show.
			onarchived();
		}}
	/>
{/if}
