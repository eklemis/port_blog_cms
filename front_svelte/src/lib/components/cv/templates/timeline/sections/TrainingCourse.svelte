<script lang="ts">
	let { designFont, displaySetting, trainingCourse, sectionCreatedHandler } = $props();
	import { onMount } from 'svelte';
	let container: HTMLElement;
	onMount(() => {
		const MOST_LEFT = 32;
		let scrollHeight = container.scrollHeight;
		const offsetLeft = container.offsetLeft;
		// specific calculation for column 2 above
		if (offsetLeft > MOST_LEFT) {
			const STANDARD_HEIGHT = 82;
			if (scrollHeight > STANDARD_HEIGHT) {
				scrollHeight = scrollHeight - STANDARD_HEIGHT;
			} else {
				scrollHeight = 0;
			}
		}
		console.log('Left:', offsetLeft);
		console.log('Training ' + trainingCourse.id + ' created!');
		console.log('scrollHeight', scrollHeight);
		sectionCreatedHandler({
			type: 'Training->' + trainingCourse.title,
			scrollHeight
		});
	});
</script>

<div
	class="flex w-1/3 flex-col rounded-sm border border-transparent p-2.5 pb-0 pl-0 hover:border-blue-500"
	bind:this={container}
>
	<h3 class={`text-${designFont.secondaryColor} text-base font-bold`}>{trainingCourse.title}</h3>
	{#if displaySetting.showInstitution && displaySetting.showPeriod}
		<p>{trainingCourse.institution}, {trainingCourse.period}</p>
	{:else if displaySetting.showInstitution}
		<p>{trainingCourse.institution}</p>
	{:else if displaySetting.showPeriod}
		<p>{trainingCourse.period}</p>
	{/if}
</div>
