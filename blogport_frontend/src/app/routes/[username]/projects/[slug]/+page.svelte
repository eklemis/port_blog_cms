<script lang="ts">
	import { page } from '$app/state';
	import { PublicProjectPage } from '$lib/pages/public-project';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();

	const canonical = $derived(new URL(page.url.pathname, page.url.origin).href);

	/**
	 * The description is markdown and the meta description is not, so the tags
	 * take the tech stack rather than the body: a summary with `##` in it reads
	 * as a mistake in a search result.
	 */
	const description = $derived(
		data.project.techStack.length
			? `${data.project.title} — ${data.project.techStack.join(', ')}.`
			: data.project.title
	);
</script>

<svelte:head>
	<title>{data.project.title} · {data.author.fullName}</title>
	<meta name="description" content={description} />
	<link rel="canonical" href={canonical} />
	<meta property="og:type" content="article" />
	<meta property="og:title" content={data.project.title} />
	<meta property="og:description" content={description} />
	<meta property="og:url" content={canonical} />
	{#if data.images.length}
		<meta property="og:image" content={data.images[0].src} />
	{/if}
</svelte:head>

<PublicProjectPage
	author={data.author}
	project={data.project}
	bodyHtml={data.bodyHtml}
	images={data.images}
/>
