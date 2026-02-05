<script lang="ts">
	import { onMount } from 'svelte';
	let { designFont, title, titleCreatedHandler } = $props();

	let container: HTMLElement;
	onMount(() => {
		let scrollHeight = container.scrollHeight;
		console.log('Title ' + title + ' created!');
		console.log('scrollHeight', scrollHeight);
		titleCreatedHandler({
			type: 'Title->' + title,
			scrollHeight
		});
	});
</script>

<h2 class={`text-${designFont.primaryColor} pt-8 text-xl font-bold`} bind:this={container}>
	{title.toUpperCase()}
</h2>
