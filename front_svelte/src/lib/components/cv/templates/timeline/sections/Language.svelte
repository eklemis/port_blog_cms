<script lang="ts">
	let { designFont, displaySetting, language, sectionCreatedHandler } = $props();
	import { onMount } from 'svelte';

	let container: HTMLElement;
	const sliderItemIndex = [...Array(language.proficiencySlider.valueMax).keys()];

	onMount(() => {
		let scrollHeight = container.scrollHeight;
		console.log('Language ' + language.id + ' created!');
		console.log('scrollHeight', scrollHeight);
		sectionCreatedHandler({
			type: 'Experience->' + language.name,
			scrollHeight
		});
	});
</script>

<div class="flex gap-x-8" bind:this={container}>
	<div>
		<p class="text-sm">{language.name}</p>
		{#if displaySetting.showProficiencyLabel}
			<p>{language.proficiencyLabel}</p>
		{/if}
	</div>
	{#if displaySetting.showProficiencySlider}
		<ul class="flex gap-x-1">
			{#if displaySetting.sliderStyle === 'Battery'}
				{#each sliderItemIndex as index}
					{#if index + 1 > language.proficiencySlider.valueAt}
						<li class="h-8 w-[6px] rounded-xs bg-gray-400"></li>
					{:else}
						<li class={`h-8 w-[6px] rounded-xs bg-${designFont.secondaryColor}`}></li>
					{/if}
				{/each}
			{/if}
		</ul>
		<p>{language.proficiencyLevel}</p>
	{/if}
</div>
