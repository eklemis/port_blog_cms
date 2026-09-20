<script lang="ts">
	import { invalidateAll } from '$app/navigation';
	import { ProjectEditorPage } from '$lib/pages/project-editor';
	import type { PageProps } from './$types';

	/** `/studio/projects/[id]` — Console Blueprint §03. */
	let { data }: PageProps = $props();
</script>

<svelte:head>
	<title>{data.project ? data.project.title : 'Project'}</title>
</svelte:head>

<ProjectEditorPage
	project={data.project}
	username={data.user.username}
	denied={data.denied}
	screenshots={data.screenshots}
	availableTopics={data.availableTopics}
	onchanged={() => invalidateAll()}
/>
