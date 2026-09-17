<script lang="ts">
	import { page } from '$app/state';
	import { PublicPostPage } from '$lib/pages/public-post';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();

	/**
	 * §03: "a shared link is a real page with real metadata — not a shell that
	 * fetches after paint". The excerpt is the description the author already
	 * wrote for exactly this (27:41: "Plain text. Used as the meta description
	 * on the public page").
	 */
	const description = $derived(data.post.excerpt ?? '');
	const canonical = $derived(new URL(page.url.pathname, page.url.origin).href);
</script>

<svelte:head>
	<title>{data.post.title} · {data.author.fullName}</title>
	{#if description}
		<meta name="description" content={description} />
		<meta property="og:description" content={description} />
	{/if}
	<link rel="canonical" href={canonical} />
	<meta property="og:type" content="article" />
	<meta property="og:title" content={data.post.title} />
	<meta property="og:url" content={canonical} />
	{#if data.cover}
		<meta property="og:image" content={data.cover.src} />
	{/if}
</svelte:head>

<PublicPostPage
	author={data.author}
	post={data.post}
	topics={data.topics}
	cover={data.cover}
	bodyHtml={data.bodyHtml}
	readMinutes={data.readMinutes}
/>
