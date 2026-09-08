<script lang="ts">
	import { goto } from '$app/navigation';
	import { NewPostPage } from '$lib/pages/new-post';
	import { CONSOLE_ROUTES } from '$lib/shared/config/routes';

	/**
	 * `/studio/posts/new` — Console Blueprint §03, J4 step one.
	 *
	 * The editor is in the surface map and not built yet, and `resolve()` only
	 * takes a route id that exists — which is what the disable below is for.
	 * Swap it in when `/studio/posts/[id]` lands.
	 */

	// eslint-disable-next-line svelte/no-navigation-without-resolve -- see above
	const openEditor = (id: string) => goto(`${CONSOLE_ROUTES.posts}/${id}`);
</script>

<svelte:head>
	<title>New post</title>
</svelte:head>

<NewPostPage oncreated={openEditor} />
