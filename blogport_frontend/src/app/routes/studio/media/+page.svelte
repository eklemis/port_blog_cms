<script lang="ts">
	import { goto, invalidateAll } from '$app/navigation';
	import { page } from '$app/state';
	import { MediaPage } from '$lib/pages/media';
	import type { PageProps } from './$types';

	/** `/studio/media` — Console Blueprint §03, scoped by attachment target. */
	let { data }: PageProps = $props();

	function choose(target: string) {
		// eslint-disable-next-line svelte/no-navigation-without-resolve -- one query parameter
		goto(`${page.url.pathname}?target=${encodeURIComponent(target)}`, {
			keepFocus: true,
			noScroll: true
		});
	}
</script>

<svelte:head>
	<title>Media</title>
</svelte:head>

<MediaPage
	items={data.items}
	scope={data.scope}
	onquery={choose}
	onchanged={() => invalidateAll()}
/>
