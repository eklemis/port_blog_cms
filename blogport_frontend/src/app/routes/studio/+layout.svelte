<script lang="ts">
	import type { Snippet } from 'svelte';
	import { page } from '$app/state';
	import { StudioShell } from '$lib/widgets/studio-shell';
	import type { LayoutProps } from './$types';

	/**
	 * Every console screen sits in this frame, and the gate is in
	 * +layout.server.ts beside it — one answer for all of them.
	 */
	let { children }: LayoutProps & { children: Snippet } = $props();
</script>

<StudioShell path={page.url.pathname}>
	{@render children()}
</StudioShell>
