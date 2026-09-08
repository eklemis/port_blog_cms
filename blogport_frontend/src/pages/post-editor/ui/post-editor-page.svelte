<script lang="ts">
	import { PostEditor } from '$lib/features/edit-post';
	import { NO_ACCESS } from '$lib/features/edit-post';
	import { Button, EmptyState } from '$lib/shared/ui';
	import { CONSOLE_ROUTES } from '$lib/shared/config/routes';

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

	let { post, denied = false }: { post: Post | null; denied?: boolean } = $props();
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
	<PostEditor {post} />
{/if}
