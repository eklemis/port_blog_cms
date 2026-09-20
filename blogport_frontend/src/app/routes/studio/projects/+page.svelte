<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { ProjectsPage } from '$lib/pages/projects';
	import type { PageProps } from './$types';

	/** `/studio/projects` — Console Blueprint §03. */
	let { data }: PageProps = $props();

	/**
	 * Every filter writes to the URL rather than to component state, so a
	 * filtered list is a link somebody can send, and the back button undoes a
	 * filter the way it undoes everything else.
	 */
	function query(changes: Record<string, string | null>) {
		// Plain objects rather than a mutable URLSearchParams: the lint rule is
		// right that a mutable built-in in a component is a trap, and the posts
		// list already answers this the same way.
		const next: Record<string, string | null> = {
			...Object.fromEntries(page.url.searchParams),
			...changes
		};

		const search = Object.entries(next)
			.filter(([, value]) => value !== null && value !== '')
			.map(([key, value]) => `${encodeURIComponent(key)}=${encodeURIComponent(String(value))}`)
			.join('&');

		// eslint-disable-next-line svelte/no-navigation-without-resolve
		goto(search ? `?${search}` : page.url.pathname, { keepFocus: true, noScroll: true });
	}
</script>

<svelte:head>
	<title>Projects</title>
</svelte:head>

<ProjectsPage
	projects={data.projects}
	topics={data.topics}
	total={data.total}
	everything={data.everything}
	page={data.page}
	perPage={data.perPage}
	filtered={data.filtered}
	failed={data.failed}
	search={data.search}
	topic={data.topic}
	sort={data.sort}
	onquery={query}
/>
