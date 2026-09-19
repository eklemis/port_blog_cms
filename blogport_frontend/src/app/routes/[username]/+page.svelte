<script lang="ts">
	import { page } from '$app/state';
	import { PublicProfilePage } from '$lib/pages/public-profile';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();

	const canonical = $derived(new URL(page.url.pathname, page.url.origin).href);
	const description = $derived(
		data.author.bio ?? `${data.author.fullName}'s writing and projects.`
	);
</script>

<svelte:head>
	<title>{data.author.fullName}</title>
	<meta name="description" content={description} />
	<link rel="canonical" href={canonical} />
	<meta property="og:type" content="profile" />
	<meta property="og:title" content={data.author.fullName} />
	<meta property="og:description" content={description} />
	<meta property="og:url" content={canonical} />
	{#if data.author.avatarSrc}
		<meta property="og:image" content={data.author.avatarSrc} />
	{/if}
</svelte:head>

<PublicProfilePage author={data.author} projects={data.projects} posts={data.posts} />
