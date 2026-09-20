<script lang="ts">
	import { goto } from '$app/navigation';
	import { NewProjectPage } from '$lib/pages/new-project';
	import { CONSOLE_ROUTES } from '$lib/shared/config/routes';

	/**
	 * `/studio/projects/new`.
	 *
	 * A static route beside `[id]`, which is the whole point: without it,
	 * `/studio/projects/new` matched `[id]` with an id of "new" and the editor
	 * told someone the project they were about to create could not be found.
	 */

	// eslint-disable-next-line svelte/no-navigation-without-resolve -- the id arrives at runtime
	const openEditor = (id: string) => goto(`${CONSOLE_ROUTES.projects}/${id}`);
</script>

<svelte:head>
	<title>New project</title>
</svelte:head>

<NewProjectPage oncreated={openEditor} />
