<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { CONSOLE_ROUTES } from '$lib/shared/config/routes';
	import { ResumesPage } from '$lib/pages/resumes';
	import type { PageProps } from './$types';

	/** `/studio/resumes` — Console Blueprint §03, J6. */
	let { data }: PageProps = $props();

	/**
	 * The search and the sort write to the URL rather than to component state,
	 * so a filtered list is a link somebody can send, and the back button undoes
	 * a search the way it undoes everything else.
	 *
	 * Assembled the projects list's way, plain objects rather than a mutable
	 * URLSearchParams — two lists doing the same thing differently is how one of
	 * them ends up with a bug the other does not.
	 */
	function query(changes: Record<string, string | null>) {
		const next: Record<string, string | null> = {
			...Object.fromEntries(page.url.searchParams),
			...changes
		};

		const search = Object.entries(next)
			.filter(([, value]) => value !== null && value !== '')
			.map(([key, value]) => `${encodeURIComponent(key)}=${encodeURIComponent(String(value))}`)
			.join('&');

		// eslint-disable-next-line svelte/no-navigation-without-resolve
		goto(search ? `?${search}` : page.url.pathname, { keepFocus: true, noScroll: true });
	}

	/** The document exists; the builder is where it gets written. J6. */
	function opened(id: string) {
		// The id arrives at runtime, already escaped; resolve() would encode it twice.
		// eslint-disable-next-line svelte/no-navigation-without-resolve
		goto(`${CONSOLE_ROUTES.resumes}/${encodeURIComponent(id)}`);
	}
</script>

<svelte:head>
	<title>Résumés</title>
</svelte:head>

<ResumesPage
	resumes={data.resumes}
	total={data.total}
	everything={data.everything}
	page={data.page}
	perPage={data.perPage}
	filtered={data.filtered}
	failed={data.failed}
	search={data.search}
	sort={data.sort}
	displayName={page.data.user?.full_name ?? ''}
	onquery={query}
	oncreated={opened}
/>
