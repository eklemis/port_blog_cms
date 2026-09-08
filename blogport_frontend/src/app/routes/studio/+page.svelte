<script lang="ts">
	import { OverviewPage } from '$lib/pages/overview';
	import type { PageProps } from './$types';

	/** `/studio` — Console Blueprint §03. The console's front door. */
	let { data }: PageProps = $props();
</script>

<svelte:head>
	<title>Overview</title>
</svelte:head>

<OverviewPage fullName={data.user.full_name} counts={data.counts} />
