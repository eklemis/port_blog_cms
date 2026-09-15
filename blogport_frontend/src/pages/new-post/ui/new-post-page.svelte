<script lang="ts">
	import { resolve } from '$app/paths';
	import { CreatePostForm } from '$lib/features/create-post';

	/**
	 * `/studio/posts/new`.
	 *
	 * Design: Screen / New post 27:2 — a breadcrumb, the title, and one card.
	 * The breadcrumb is the desktop's way back; a phone's is the back chevron in
	 * the shell's header, per Mobile / New post 97:2871.
	 */
	let { oncreated }: { oncreated: (id: string) => void } = $props();
</script>

<div class="flex flex-col gap-[18px]">
	<nav aria-label="Breadcrumb" class="max-md:hidden">
		<ol class="flex items-center gap-[7px] text-[12px]">
			<li>
				<a href={resolve('/studio/posts')} class="text-arch-accent-ink hover:underline">Posts</a>
			</li>
			<li aria-hidden="true" class="text-arch-muted">/</li>
			<li aria-current="page" class="text-arch-muted">New</li>
		</ol>
	</nav>

	<h1
		class="font-display text-[24px] font-extrabold tracking-tight text-arch-headline max-md:sr-only md:text-[27px]"
	>
		New post
	</h1>

	<CreatePostForm {oncreated} />
</div>
