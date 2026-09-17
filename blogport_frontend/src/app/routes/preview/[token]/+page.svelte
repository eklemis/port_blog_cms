<script lang="ts">
	import { DraftPreviewPage } from '$lib/pages/draft-preview';
	import type { PageProps } from './$types';

	/** `/preview/[token]` — Console Blueprint §03, public surfaces. */
	let { data }: PageProps = $props();
</script>

<svelte:head>
	<title>{data.post.title}</title>
	<!-- A draft behind a bearer token: it must never be indexed, whatever the
	     crawler was handed. -->
	<meta name="robots" content="noindex, nofollow" />
</svelte:head>

<DraftPreviewPage post={data.post} />
