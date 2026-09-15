<script lang="ts">
	import { goto, invalidateAll } from '$app/navigation';
	import { navigating, page } from '$app/state';
	import { PostsArchivePage } from '$lib/pages/posts-archive';
	import type { PageProps } from './$types';

	/** `/studio/posts/archive` — Console Blueprint §03 and §06's destruction rungs. */
	let { data }: PageProps = $props();

	function openPage(next: number) {
		const query = next > 1 ? `?page=${next}` : page.url.pathname;
		// A query string on the current route, which resolve() has no form for.
		// eslint-disable-next-line svelte/no-navigation-without-resolve
		goto(query, { noScroll: true });
	}
</script>

<svelte:head>
	<title>Archived posts</title>
</svelte:head>

<PostsArchivePage
	posts={data.posts}
	total={data.total}
	page={data.page}
	perPage={data.perPage}
	failed={data.failed}
	loading={Boolean(navigating.to)}
	onchanged={() => invalidateAll()}
	onpage={openPage}
/>
