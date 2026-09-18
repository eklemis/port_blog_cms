<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { PublicProjectsPage } from '$lib/pages/public-projects';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();

	const canonical = $derived(new URL(page.url.pathname, page.url.origin).href);
	const description = $derived(`Projects built by ${data.author.fullName}.`);

	/** Paging keeps the topic and goes back through the server. */
	function toPage(next: number) {
		const topic = page.url.searchParams.get('topic_id');
		const parts = [
			topic ? `topic_id=${encodeURIComponent(topic)}` : '',
			next > 1 ? `page=${next}` : ''
		].filter(Boolean);

		const query = parts.join('&');
		// The path is this page's own, carrying a username that arrived at
		// runtime; there is no route id to resolve that would not re-encode it.
		// eslint-disable-next-line svelte/no-navigation-without-resolve
		goto(query ? `${page.url.pathname}?${query}` : page.url.pathname);
	}
</script>

<svelte:head>
	<title>Projects · {data.author.fullName}</title>
	<meta name="description" content={description} />
	<link rel="canonical" href={canonical} />
	<meta property="og:type" content="profile" />
	<meta property="og:title" content="Projects · {data.author.fullName}" />
	<meta property="og:description" content={description} />
	<meta property="og:url" content={canonical} />
</svelte:head>

<PublicProjectsPage
	author={data.author}
	projects={data.projects}
	filter={data.filter}
	total={data.total}
	page={data.page}
	perPage={data.perPage}
	onpage={toPage}
/>
