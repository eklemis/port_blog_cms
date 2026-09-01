<script lang="ts">
	import { resolve } from '$app/paths';
	import LatestBlogs from '$lib/widgets/latest-blogs/ui/latest-blogs.svelte';
	import FeaturedProjects from '$lib/widgets/featured-projects/ui/featured-projects.svelte';

	const latestBlogs = [
		{
			id: 1,
			title: "Designing Decision-Ready Data Systems",
			excerpt: "Why secure architecture matters more than dashboards.",
			author: "Jane Doe"
		},
		{
			id: 2,
			title: "From Raw Logs to Reliable Insight",
			excerpt: "Turning messy operational data into measurable improvements.",
			author: "David Chen"
		}
	];

	const featuredProjects = [
		{
			id: 1,
			title: "Operational Data Governance Framework",
			description: "A scalable architecture improving cross-team data reliability."
		},
		{
			id: 2,
			title: "Secure Resume Platform API",
			description: "Role-based access system for multi-tenant content publishing."
		}
	];
</script>

<section class="bg-arch-bg text-arch-headline">

	<!-- Hero -->
	<div class="max-w-6xl mx-auto px-6 py-28 text-center space-y-6">
		<h1 class="font-display text-5xl md:text-6xl font-extrabold leading-tight">
			Build your professional identity
			<span class="block text-(--color-arch-accent)">
				with clarity and confidence.
			</span>
		</h1>

		<p class="text-lg text-arch-muted max-w-2xl mx-auto">
			Create a structured resume, showcase your projects,
			and publish insights that reflect your real capabilities —
			all in one place.
		</p>

		<div class="pt-6 flex justify-center gap-4">
			<a
				href={resolve('/auth/register')}
				class="px-6 py-3 rounded-lg font-semibold bg-(--color-arch-accent) text-black hover:brightness-95 transition">
				Get Started
			</a>

			<a
				href={resolve('/blog')}
				class="px-6 py-3 rounded-lg border border-white/10 text-arch-muted hover:border-white/20 transition">
				Explore Blogs
			</a>
		</div>
	</div>

	<LatestBlogs blogs={latestBlogs} />
	<FeaturedProjects projects={featuredProjects} />

</section>
