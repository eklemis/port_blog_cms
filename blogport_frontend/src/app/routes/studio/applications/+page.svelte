<script lang="ts">
	import { navigating } from '$app/state';
	import { ApplicationsPage } from '$lib/pages/applications';
	import type { PageProps } from './$types';

	/** `/studio/applications` — Console Blueprint §03, Career Studio. */
	let { data }: PageProps = $props();
</script>

<svelte:head>
	<title>Applications</title>
</svelte:head>

<ApplicationsPage rows={data.rows} failed={data.failed} loading={Boolean(navigating.to)} />
