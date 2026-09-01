<script lang="ts">
	export let project: {
		id: number;
		title: string;
		description: string;
	};
</script>

<div class="bg-arch-surface p-6 rounded-xl border border-white/5">
	<h3 class="font-(--font-display) text-xl mb-2">
		{project.title}
	</h3>

	<p class="text-arch-muted text-sm">
		{project.description}
	</p>
</div>
