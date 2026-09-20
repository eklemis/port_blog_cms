<script lang="ts">
	import { resolve } from '$app/paths';
	import { CreateProjectForm } from '$lib/features/create-project';

	/** `/studio/projects/new` — Console Blueprint §03, the Add project journey. */
	let { oncreated }: { oncreated: (id: string) => void } = $props();
</script>

<div class="flex flex-col gap-[18px]">
	<nav aria-label="Breadcrumb" class="max-md:hidden">
		<ol class="flex items-center gap-[7px] text-[12px]">
			<li>
				<a href={resolve('/studio/projects')} class="text-arch-accent-ink hover:underline">
					Projects
				</a>
			</li>
			<li aria-hidden="true" class="text-arch-muted">/</li>
			<li aria-current="page" class="text-arch-muted">New</li>
		</ol>
	</nav>

	<h1
		class="font-display text-[24px] font-extrabold tracking-tight text-arch-headline
		       max-md:sr-only md:text-[27px]"
	>
		New project
	</h1>

	<CreateProjectForm {oncreated} />
</div>
