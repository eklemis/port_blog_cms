<script lang="ts">
	import { CreatePostForm } from '$lib/features/create-post';

	/**
	 * `/studio/posts/new`.
	 *
	 * One column, one form. J4's first step is deliberately small — the post
	 * exists so that everything else can attach to it.
	 *
	 * Design: Screen / New post.
	 */
	let { oncreated }: { oncreated: (id: string) => void } = $props();
</script>

<div class="flex max-w-[620px] flex-col gap-5">
	<div class="flex flex-col gap-1.5">
		<h1
			class="font-display text-[24px] font-extrabold tracking-tight text-arch-headline md:text-[27px]"
		>
			New post
		</h1>
		<p class="text-[13px] text-arch-muted">
			It starts as a draft. Nothing is public until you publish it.
		</p>
	</div>

	<CreatePostForm {oncreated} />
</div>
