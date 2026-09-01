<script lang="ts">
	import ProjectCard from '$lib/entities/project/ui/project-card.svelte';

	export let projects: {
		id: number;
		title: string;
		description: string;
	}[];
</script>

<section class="max-w-6xl mx-auto px-6 py-16 border-t border-white/5">
	<h2 class="font-(--font-display) text-3xl mb-10">
		Featured Projects
	</h2>

	<div class="grid md:grid-cols-2 gap-8">
		{#each projects as project (project.id)}
			<ProjectCard {project} />
		{/each}
	</div>
</section>
