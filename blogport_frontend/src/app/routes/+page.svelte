<script lang="ts">
	import { Button } from '$lib/shared/ui';
</script>

<main class="mx-auto flex min-h-screen max-w-2xl flex-col justify-center gap-6 p-8">
	<p class="font-mono text-xs tracking-[0.16em] text-arch-accent-ink uppercase">Blogport</p>
	<h1 class="font-display text-4xl font-extrabold tracking-tight">The frontend starts here.</h1>
	<p class="max-w-prose text-arch-muted">
		Nothing is built yet, and that is deliberate — the previous implementation was removed so this
		one can follow the design rather than be retrofitted to it. The route map is in the Console
		Blueprint; build against it, one slice at a time, test first.
	</p>
	<div class="flex gap-3">
		<Button label="Read the blueprint" />
		<Button label="Component library" kind="secondary" />
	</div>
</main>
