<script lang="ts">
	import BlogCard from '$lib/entities/blog/ui/blog-card.svelte';

	export let blogs: {
		id: number;
		title: string;
		excerpt: string;
		author: string;
	}[];
</script>

<section class="max-w-6xl mx-auto px-6 py-16">
	<h2 class="font-(--font-display) text-3xl mb-10">
		Latest Insights
	</h2>

	<div class="grid md:grid-cols-2 gap-8">
		{#each blogs as blog (blog.id)}
			<BlogCard {blog} />
		{/each}
	</div>
</section>
