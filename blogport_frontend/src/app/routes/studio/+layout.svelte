<script lang="ts">
	import type { Snippet } from 'svelte';
	import { page } from '$app/state';
	import { StudioShell, currentLabel } from '$lib/widgets/studio-shell';
	import type { LayoutProps } from './$types';

	/**
	 * Every console screen sits in this frame, and the gate is in
	 * +layout.server.ts beside it — one answer for all of them.
	 */
	let { children }: LayoutProps & { children: Snippet } = $props();

	/**
	 * Named on the mobile bar, where the sidebar is not there to say it. Derived
	 * from the nav rather than declared per page, so the bar cannot name one
	 * screen while the tab bar lights another.
	 */
	const title = $derived(currentLabel(page.url.pathname));
</script>

<StudioShell path={page.url.pathname} {title}>
	{@render children()}
</StudioShell>
