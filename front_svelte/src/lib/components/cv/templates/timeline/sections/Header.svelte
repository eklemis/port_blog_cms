<script lang="ts">
	import AppIcon from '$lib/icons/appui/AppIcon.svelte';
	let { designFont, displaySetting, header, sectionCreatedHandler } = $props();
	import { onMount } from 'svelte';

	let container: HTMLElement;
	onMount(() => {
		let scrollHeight = container.scrollHeight;
		console.log('Header ' + header.title + ' created!');
		console.log('scrollHeight', scrollHeight);
		sectionCreatedHandler({
			type: 'Header->' + header.name,
			secIndex: 0,
			subIndex: 0,
			scrollHeight
		});
	});
</script>

<section
	class="-ml-3 flex w-full items-center gap-y-2 rounded-sm border border-gray-500 p-2.5 hover:border-blue-500"
	bind:this={container}
>
	<div>
		<h1 class={`text-${designFont.primaryColor} text-4xl font-bold`}>
			{displaySetting.nameUppercase ? header?.name?.toUpperCase() : header.name}
		</h1>
		<h2 class={`text-${designFont.secondaryColor} text-2xl font-bold`}>
			{header.title}
		</h2>

		<ul class="mt-2 flex max-w-2xl flex-wrap gap-x-4 gap-y-2 text-sm font-bold">
			{#if displaySetting.showPhone}
				<li class="flex items-center gap-x-1.5">
					<span
						><AppIcon name="phone" size="small" color={`text-${designFont.secondaryColor}`} /></span
					>
					<p>{header?.phone}</p>
				</li>
			{/if}
			{#if displaySetting.showEmail}
				<li class="flex items-center gap-x-1.5">
					<span
						><AppIcon name="email" size="small" color={`text-${designFont.secondaryColor}`} /></span
					>
					<p>{header?.email}</p>
				</li>
			{/if}
			{#if displaySetting.showLink}
				<li class="flex items-center gap-x-1.5">
					<span
						><AppIcon name="link" size="small" color={`text-${designFont.secondaryColor}`} /></span
					>
					<p>{header?.link}</p>
				</li>
			{/if}
			{#if displaySetting.showExtraLink}
				<li class="flex items-center gap-x-1.5">
					<span
						><AppIcon name="phone" size="small" color={`text-${designFont.secondaryColor}`} /></span
					>
					<p>{header?.extraLink}</p>
				</li>
			{/if}
			{#if displaySetting.showLocation}
				<li class="flex items-center gap-x-1.5">
					<span
						><AppIcon
							name="location"
							size="small"
							color={`text-${designFont.secondaryColor}`}
						/></span
					>
					<p>{header?.location}</p>
				</li>
			{/if}
			{#if displaySetting.showExtraField}
				<li class="flex items-center gap-x-1.5">
					<span
						><AppIcon name="phone" size="small" color={`text-${designFont.secondaryColor}`} /></span
					>
					<p>{header?.extraField}</p>
				</li>
			{/if}
		</ul>
	</div>
	{#if displaySetting.showPhoto}
		<div class="min-h-40 min-w-40 rounded border border-gray-500">Photo</div>
	{/if}
</section>
