<script lang="ts">
	import { goto } from '$app/navigation';
	import { navigating, page } from '$app/state';
	import { PostsPage } from '$lib/pages/posts';
	import type { PageProps } from './$types';

	/**
	 * `/studio/posts` — Console Blueprint §03.
	 *
	 * Filters are written to the URL rather than held in memory, so a filtered
	 * list can be shared, reloaded and stepped back out of. `keepFocus` so the
	 * search box does not lose the cursor mid-word on every debounce.
	 */
	let { data }: PageProps = $props();

	function apply(changes: Record<string, string | null>) {
		// Plain objects rather than a mutable URLSearchParams or Map: the lint
		// rule is right that a mutable built-in in a component is a trap, and
		// nothing here needs one.
		const next: Record<string, string | null> = {
			...Object.fromEntries(page.url.searchParams),
			...changes
		};

		const query = Object.entries(next)
			.filter(([, value]) => value !== null && value !== '')
			.map(([key, value]) => `${encodeURIComponent(key)}=${encodeURIComponent(String(value))}`)
			.join('&');

		// eslint-disable-next-line svelte/no-navigation-without-resolve
		goto(query ? `?${query}` : page.url.pathname, { keepFocus: true, noScroll: true });
	}
</script>

<svelte:head>
	<title>Posts</title>
</svelte:head>

<PostsPage
	posts={data.posts}
	total={data.total}
	page={data.page}
	perPage={data.perPage}
	filtered={data.filtered}
	failed={data.failed}
	loading={Boolean(navigating.to)}
	search={data.search}
	published={data.published}
	sort={data.sort}
	onquery={apply}
/>
