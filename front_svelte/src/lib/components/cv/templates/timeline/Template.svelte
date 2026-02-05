<script lang="ts">
	import { ESectionTypes } from '$lib/types/section_types';
	import type { ISection } from '$lib/types/section_types';
	import Page from './Page.svelte';

	let { designFont, sections } = $props();

	let allSections: ISection[] = $state<ISection[]>([
		{
			sectionType: ESectionTypes.Header,
			sectionTitle: 'Header',
			data: {
				rows: sections.header.content,
				displaySetting: sections.header.displaySetting
			},
			column: 1
		},
		{
			sectionType: ESectionTypes.Summary,
			sectionTitle: 'Summary',
			data: {
				rows: sections.summary.content,
				displaySetting: sections.summary.displaySetting
			},
			column: 1
		},
		{
			sectionType: ESectionTypes.Experience,
			sectionTitle: 'Experience',
			data: {
				rows: sections.experience.content,
				displaySetting: sections.experience.displaySetting
			},
			column: 1
		},
		{
			sectionType: ESectionTypes.Project,
			sectionTitle: 'Projects',
			data: {
				rows: sections.projects.content,
				displaySetting: sections.projects.displaySetting
			},
			column: 1
		},
		{
			sectionType: ESectionTypes.Publication,
			sectionTitle: 'Publication',
			data: {
				rows: sections.publication.content,
				displaySetting: sections.publication.displaySetting
			},
			column: 1
		},
		{
			sectionType: ESectionTypes.Volunteering,
			sectionTitle: 'Volunteering',
			data: {
				rows: sections.volunteerings.content,
				displaySetting: sections.volunteerings.displaySetting
			},
			column: 1
		},
		{
			sectionType: ESectionTypes.TrainingCourse,
			sectionTitle: 'Training/Courses',
			data: {
				rows: sections.trainingCourses.content,
				displaySetting: sections.trainingCourses.displaySetting
			},
			column: 1
		},
		{
			sectionType: ESectionTypes.Skill,
			sectionTitle: 'Skills',
			data: {
				rows: sections.skills.content,
				displaySetting: sections.skills.displaySetting
			},
			column: 1
		},
		{
			sectionType: ESectionTypes.Language,
			sectionTitle: 'Languages',
			data: {
				rows: sections.languages.content,
				displaySetting: sections.languages.displaySetting
			},
			column: 1
		}
	]);

	let pages = $state([{ startIndex: 0, startSubIndex: 0 }]);

	function throwPageOverFlow(atIndex: number, atSubIndex: number) {
		console.log('Page overflow happened-> atIndex', atIndex, ', subIndex:', atSubIndex);
		console.log('All sections: ', $state.snapshot(allSections));
		pages.push({ startIndex: atIndex, startSubIndex: atSubIndex });
		console.log('New page setups:', $state.snapshot(pages));
	}
</script>

<div class="font-inter flex flex-wrap gap-8 text-xs antialiased">
	{#each pages as page, pIdx ('cv-page-' + page.startIndex + '' + page.startSubIndex)}
		{#if pIdx > 0}
			<span class="ml-4 flex w-[794px] items-center justify-center">Page {pIdx + 1}</span>
		{/if}
		<Page
			{allSections}
			startIndex={page.startIndex}
			startSubIndex={page.startSubIndex}
			{designFont}
			{throwPageOverFlow}
		/>
	{/each}
</div>
