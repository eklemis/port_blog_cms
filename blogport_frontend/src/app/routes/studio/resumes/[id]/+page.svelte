<script lang="ts">
	import { invalidateAll } from '$app/navigation';
	import { CvBuilderPage } from '$lib/pages/cv-builder';
	import type { PageProps } from './$types';

	/** `/studio/resumes/[id]` — Console Blueprint §03, J6. */
	let { data }: PageProps = $props();
</script>

<svelte:head>
	<title>{data.cv ? data.cv.role || 'Résumé' : 'Résumé'}</title>
</svelte:head>

<CvBuilderPage
	cv={data.cv}
	projects={data.projects}
	denied={data.denied}
	onsaved={() => invalidateAll()}
/>
