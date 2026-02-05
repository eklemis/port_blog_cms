<script lang="ts">
	let { designFont, displaySetting, skill, sectionCreatedHandler } = $props();
	import { onMount } from 'svelte';
	let container: HTMLElement;
	onMount(() => {
		const MOST_LEFT = 32;
		let scrollHeight = container.scrollHeight;
		const offsetLeft = container.offsetLeft;
		// specific calculation for column 2 above
		if (offsetLeft > MOST_LEFT) {
			const STANDARD_HEIGHT = 38;
			if (scrollHeight > STANDARD_HEIGHT) {
				scrollHeight = scrollHeight - STANDARD_HEIGHT;
			} else {
				scrollHeight = 0;
			}
		}
		console.log('Left:', offsetLeft);
		console.log('Skill ' + skill.group + skill.name + ' created!');
		console.log('scrollHeight', scrollHeight);
		sectionCreatedHandler({
			type: 'Skill->' + skill.name,
			scrollHeight
		});
	});
</script>

<li
	class="flex w-auto flex-col items-center justify-center border-b border-b-slate-700 p-2.5 pb-1 text-center text-sm/6"
	bind:this={container}
>
	<span class="w-full text-center font-bold text-slate-700">{skill.name}</span>
</li>
