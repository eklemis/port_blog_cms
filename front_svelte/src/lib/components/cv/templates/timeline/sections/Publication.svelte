<script lang="ts">
	let { designFont, displaySetting, publication, sectionCreatedHandler } = $props();
	import { onMount } from 'svelte';
	let container: HTMLElement;
	onMount(() => {
		let scrollHeight = container.scrollHeight;
		console.log('Publication ' + publication.id + ' created!');
		console.log('scrollHeight', scrollHeight);
		sectionCreatedHandler({
			type: 'Publication->' + publication.title,
			scrollHeight
		});
	});
</script>

<div
	class="-ml-3 flex w-full flex-col gap-y-2 rounded-sm border border-transparent p-2.5 hover:border-blue-500"
	bind:this={container}
>
	<h3 class={`text-${designFont.primaryColor} text-lg`}>{publication.title}</h3>
	{#if displaySetting.showPublisher}
		<h4 class={`text-${designFont.secondaryColor} text-base font-bold`}>{publication.publisher}</h4>
	{/if}
</div>
