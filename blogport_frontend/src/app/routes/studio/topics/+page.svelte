<script lang="ts">
	import { invalidateAll } from '$app/navigation';
	import { TopicsPage } from '$lib/pages/topics';
	import type { PageProps } from './$types';

	/** `/studio/topics` — Console Blueprint §03, J9. */
	let { data }: PageProps = $props();
</script>

<svelte:head>
	<title>Topics</title>
</svelte:head>

<TopicsPage topics={data.topics} onchanged={() => invalidateAll()} />
