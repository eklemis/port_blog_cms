<script lang="ts">
	export let blog: {
		id: number;
		title: string;
		excerpt: string;
		author: string;
	};
</script>

<div class="bg-arch-surface p-6 rounded-xl border border-white/5">
	<h3 class="font-(--font-display) text-xl mb-2">
		{blog.title}
	</h3>

	<p class="text-arch-muted text-sm mb-4">
		{blog.excerpt}
	</p>

	<p class="text-xs text-arch-muted">
		By {blog.author}
	</p>
</div>
