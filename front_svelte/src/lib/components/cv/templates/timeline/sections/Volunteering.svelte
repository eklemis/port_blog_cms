<script lang="ts">
	import AppIcon from '$lib/icons/appui/AppIcon.svelte';
	let { designFont, displaySetting, volunteering, sectionCreatedHandler } = $props();
	import { onMount } from 'svelte';
	let container: HTMLElement;
	onMount(() => {
		let scrollHeight = container.scrollHeight;

		console.log('Volunteering ' + volunteering.id + ' created!');
		console.log('scrollHeight', scrollHeight);

		sectionCreatedHandler({
			type: 'Volunteering->' + volunteering.title,
			scrollHeight
		});
	});
</script>

<div
	class="-ml-3 box-border flex w-full flex-col gap-y-2 rounded-sm border border-transparent p-2.5 hover:border-blue-500"
	bind:this={container}
>
	<h3 class={`text-${designFont.primaryColor} text-lg`}>{volunteering.title}</h3>
	<ul class="flex max-w-2xl flex-wrap gap-x-4 gap-y-2">
		{#if displaySetting.showPeriod}
			<li class="flex items-center gap-x-1.5">
				<span><AppIcon name="calendar" size="small" color="text-gray-700" /></span
				>{volunteering.period}
			</li>
		{/if}
		{#if displaySetting.showLocation}
			<li class="flex items-center gap-x-1.5">
				<span><AppIcon name="location" size="small" color="text-gray-700" /></span
				>{volunteering.location}
			</li>
		{/if}
	</ul>
	{#if displaySetting.showDescription}
		<p>{volunteering.description}</p>
	{/if}
	{#if displaySetting.showBullets}
		<ul>
			{#each volunteering.bullets as bullet, bulIdx ('volbul-' + bulIdx)}
				<li>{bullet}</li>
			{/each}
		</ul>
	{/if}
</div>
