<script lang="ts">
	let { designFont, displaySetting, summary, sectionCreatedHandler } = $props();
	import { onMount } from 'svelte';
	let container: HTMLElement;
	onMount(() => {
		let scrollHeight = container.scrollHeight;
		console.log('Summary created!');
		console.log('scrollHeight', scrollHeight);
		sectionCreatedHandler({
			type: 'Summary->' + summary.description,
			scrollHeight
		});
	});
</script>

<div bind:this={container} class="pt-2">
	{#if displaySetting.showDescription}
		<p>{summary.description}</p>
	{/if}
</div>
