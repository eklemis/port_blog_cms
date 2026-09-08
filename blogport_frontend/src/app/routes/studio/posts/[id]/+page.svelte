<script lang="ts">
	import { PostEditorPage } from '$lib/pages/post-editor';
	import type { PageProps } from './$types';

	/** `/studio/posts/[id]` — Console Blueprint §03, J4 steps two and three. */
	let { data }: PageProps = $props();
</script>

<svelte:head>
	<title>{data.post ? data.post.title : 'Post'}</title>
</svelte:head>

<PostEditorPage post={data.post} username={data.user.username} denied={data.denied} />
