<script lang="ts">
	import { goto } from '$app/navigation';
	import { navigating, page } from '$app/state';
	import { ApplicationsPage } from '$lib/pages/applications';
	import type { PageProps } from './$types';

	/** `/studio/applications` — Console Blueprint §03, Career Studio. */
	let { data }: PageProps = $props();

	function openPage(next: number) {
		// The page lives in the URL, so a page of the tracker is a shareable,
		// reloadable, back-button-safe address like every other list.
		const query = next > 1 ? `?page=${next}` : page.url.pathname;

		// A query string on the current route, which resolve() has no form for.
		// eslint-disable-next-line svelte/no-navigation-without-resolve
		goto(query, { noScroll: true });
	}
</script>

<svelte:head>
	<title>Applications</title>
</svelte:head>

<ApplicationsPage
	rows={data.rows}
	total={data.total}
	page={data.page}
	perPage={data.perPage}
	failed={data.failed}
	loading={Boolean(navigating.to)}
	onpage={openPage}
/>
