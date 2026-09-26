<script lang="ts">
	import { goto, invalidateAll } from '$app/navigation';
	import { page } from '$app/state';
	import { MediaPage } from '$lib/pages/media';
	import type { PageProps } from './$types';

	/** `/studio/media` — Console Blueprint §03, scoped by attachment target. */
	let { data }: PageProps = $props();

	function choose(target: string, archived: boolean) {
		const query = `target=${encodeURIComponent(target)}${archived ? '&archived=true' : ''}`;

		// eslint-disable-next-line svelte/no-navigation-without-resolve -- query only
		goto(`${page.url.pathname}?${query}`, { keepFocus: true, noScroll: true });
	}
</script>

<svelte:head>
	<title>Media</title>
</svelte:head>

<MediaPage
	items={data.items}
	scope={data.scope}
	archived={data.archived}
	onquery={choose}
	onchanged={() => invalidateAll()}
/>
