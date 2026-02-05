<script lang="ts">
	import type { ISection } from '$lib/types/section_types';
	import { cloneSection } from '$lib/types/section_types';
	import { ESectionTypes } from '$lib/types/section_types';
	import SectionTitle from './sections/SectionTitle.svelte';
	import Header from './sections/Header.svelte';
	import Summary from './sections/Summary.svelte';
	import Experience from './sections/Experience.svelte';
	import Publication from './sections/Publication.svelte';
	import Project from './sections/Project.svelte';
	import Volunteering from './sections/Volunteering.svelte';
	import TrainingCourse from './sections/TrainingCourse.svelte';
	import Skill from './sections/Skill.svelte';
	import Language from './sections/Language.svelte';

	type throwOverFlowFunction = (atIndex: number, atSubIndex: number) => void;
	const { designFont, allSections, startIndex, startSubIndex, throwPageOverFlow } = $props<{
		designFont: object;
		allSections: ISection[];
		startIndex: number;
		startSubIndex: number;
		throwPageOverFlow: throwOverFlowFunction;
	}>();

	let container: HTMLDivElement;

	function constructFirstSection(
		sections: ISection[],
		initialIndex: number,
		initialSubIndex: number
	): ISection {
		let startSection: ISection = cloneSection(sections[initialIndex]);

		const selectedRow = startSection.data.rows[initialSubIndex];
		startSection.data.rows = Array.from([selectedRow]); //.slice(initialSubIndex, initialSubIndex + 1);

		return startSection;
	}
	let currentIndex = $state(startIndex);
	let currentSubIndex = $state(startSubIndex);

	let pageSections = $state<ISection[]>([
		constructFirstSection(allSections, startIndex, startSubIndex)
	]);

	const allMarginBorderHeight = 66; // Account for top and bottom padding plus 2px top+bottom line
	let contentHeight = allMarginBorderHeight; //$state(allMarginBorderHeight);
	let elHeights = $state<{ top: number; height: number }[]>([{ top: 0, height: 32 }]);
	const addHeight = (newHeight: number) => {
		const prevTop = elHeights[elHeights.length - 1].top;
		const prevHeight = elHeights[elHeights.length - 1].height;
		elHeights.push({ top: prevTop + prevHeight, height: newHeight });
	};

	function hasOverFlown(maxHeight: number, currentHeight: number): boolean {
		return currentHeight > maxHeight;
	}

	function addOneMoreSubSection(source: ISection[]) {
		if (currentIndex < source.length) {
			let correspondingSourceSection = source[currentIndex];
			console.log('Corresponding source section:', $state.snapshot(correspondingSourceSection));

			const sourceSectionRowsLength = correspondingSourceSection.data.rows.length;
			// move to next section only when all sub section taken
			if (currentSubIndex >= sourceSectionRowsLength - 1) {
				console.log('Last sub index reached!');
				currentIndex++;
				currentSubIndex = 0;
				if (currentIndex >= source.length) {
					console.log('Reached end section!');
					return;
				}
				let newSection = cloneSection(source[currentIndex]);
				// keep only one sub section
				try {
					const selectedRow = newSection.data.rows[currentSubIndex];
					newSection.data.rows = [selectedRow];
				} catch (er) {
					console.error(er);
					console.log('new section:', newSection);
					console.log('allSection at currIndex:', source[currentIndex]);
					console.log('Index:', currentIndex);
					console.log('All Sections:', $state.snapshot(source));
				}

				pageSections = [...pageSections, newSection];

				return;
			}
			// update the sub sections of the latest section
			// take only one sub section and move the cursor
			currentSubIndex++;
			let pageSectionLastIndex = pageSections.length - 1;
			const pageLastSectionRows = pageSections[pageSectionLastIndex].data.rows;
			pageSections[pageSectionLastIndex].data.rows = [
				...pageLastSectionRows,
				source[currentIndex].data.rows[currentSubIndex]
			];
		}
	}
	function removeLastAddedSubSection() {
		if (currentSubIndex === 0) {
			pageSections.pop();
		} else {
			pageSections[pageSections.length - 1].data.rows.pop();
		}
	}
	function titleCreatedHandler(msg: { type: string; scrollHeight: number }) {
		contentHeight = contentHeight + msg.scrollHeight;
		addHeight(msg.scrollHeight);
		console.log('Current content height:', contentHeight);
		let overFlown = false;
		if (container) {
			overFlown = hasOverFlown(container.clientHeight, contentHeight);
			if (overFlown) {
				removeLastAddedSubSection();
				throwPageOverFlow(currentIndex, currentSubIndex);
			}
		}
	}
	function sectionCreatedHandler(msg: {
		type: string;
		secIndex: number;
		subIndex: number;
		scrollHeight: number;
	}) {
		contentHeight = contentHeight + msg.scrollHeight;
		addHeight(msg.scrollHeight);
		let overFlown = false;
		if (container) {
			overFlown = hasOverFlown(container.clientHeight, contentHeight);
		}
		if (!overFlown) {
			console.log(
				'current idexes(BEFORE) --> Index:',
				currentIndex,
				', subIndex:',
				currentSubIndex
			);
			console.log('BEFORE sub section inserted: ', $state.snapshot(pageSections));
			addOneMoreSubSection(allSections);
			console.log('One more sub section inserted: ', $state.snapshot(pageSections));
			console.log('All sections is: ', $state.snapshot(allSections));
			console.log('current idexes(AFTER) --> Index:', currentIndex, ', subIndex:', currentSubIndex);
		} else {
			removeLastAddedSubSection();
			console.log('Last sub section has been deleted: ', $state.snapshot(pageSections));
			throwPageOverFlow(currentIndex, currentSubIndex);
		}
	}
</script>

<div
	class="page relative m-4 my-1 h-[1123px] min-h-[1123px] w-[794px] overflow-visible rounded p-8 [box-shadow:rgba(0,_0,_0,_0.16)_0px_10px_36px_0px,_rgba(0,_0,_0,_0.06)_0px_0px_0px_1px]"
	bind:this={container}
	id="pg-090"
>
	<div class="absolute top-[1091px] left-0 h-[1px] w-full bg-red-600"></div>
	{#each elHeights as elHeight, hIdx ('height-' + hIdx)}
		<div
			class="absolute top-[{elHeight.top}px] left-1/4 h-[{elHeight.height}px] w-6 bg-red-600"
			style="top:{elHeight.top}px;height:{elHeight.height}px"
		>
			<span
				class="absolute top-0 left-0 box-border flex h-full w-full items-center justify-center border border-white"
				>{elHeight.height}</span
			>
		</div>
	{/each}
	{#each pageSections as section, sec_idx ('section-' + sec_idx)}
		{#if section.sectionType === ESectionTypes.Header}
			<Header
				{designFont}
				displaySetting={section.data.displaySetting}
				header={section.data.rows[0]}
				{sectionCreatedHandler}
			/>
		{:else if section.sectionType === ESectionTypes.Summary}
			<SectionTitle {designFont} title={section.sectionTitle} {titleCreatedHandler} />
			<Summary
				{designFont}
				displaySetting={section.data.displaySetting}
				summary={section.data.rows[0]}
				{sectionCreatedHandler}
			/>
		{:else if section.sectionType === ESectionTypes.Experience}
			<SectionTitle {designFont} title={section.sectionTitle} {titleCreatedHandler} />
			{#each section.data.rows as exp, exp_idx ('experience-' + exp.id)}
				<Experience
					{designFont}
					displaySetting={section.data.displaySetting}
					experience={exp}
					secIndex={sec_idx}
					subIndex={exp_idx}
					{sectionCreatedHandler}
				/>
			{/each}
		{:else if section.sectionType === ESectionTypes.Project}
			<SectionTitle {designFont} title={section.sectionTitle} {titleCreatedHandler} />
			{#each section.data.rows as project ('project-' + project.id)}
				<Project
					{designFont}
					displaySetting={section.data.displaySetting}
					{project}
					{sectionCreatedHandler}
				/>
			{/each}
		{:else if section.sectionType === ESectionTypes.Publication}
			<SectionTitle {designFont} title={section.sectionTitle} {titleCreatedHandler} />
			{#each section.data.rows as pub ('pub-' + pub.id)}
				<Publication
					{designFont}
					displaySetting={section.data.displaySetting}
					publication={pub}
					{sectionCreatedHandler}
				/>
			{/each}
		{:else if section.sectionType === ESectionTypes.Volunteering}
			<SectionTitle {designFont} title={section.sectionTitle} {titleCreatedHandler} />
			{#each section.data.rows as volunteering ('volunteer-' + volunteering.id)}
				<Volunteering
					{designFont}
					displaySetting={section.data.displaySetting}
					{volunteering}
					{sectionCreatedHandler}
				/>
			{/each}
		{:else if section.sectionType === ESectionTypes.TrainingCourse}
			<section>
				<SectionTitle {designFont} title={section.sectionTitle} {titleCreatedHandler} />
				<div class="flex flex-row flex-wrap justify-between gap-y-1">
					{#each section.data.rows as trainingCourse, tcIdx ('tc-' + tcIdx)}
						<TrainingCourse
							{designFont}
							displaySetting={section.data.displaySetting}
							{sectionCreatedHandler}
							{trainingCourse}
						/>
					{/each}
				</div>
			</section>
		{:else if section.sectionType === ESectionTypes.Skill}
			<section>
				<SectionTitle {designFont} title={section.sectionTitle} {titleCreatedHandler} />
				<ul class="flex flex-row flex-wrap items-start gap-x-2 gap-y-2">
					{#each section.data.rows as skill, tcIdx ('skill-' + skill.name)}
						<Skill
							{designFont}
							displaySetting={section.data.displaySetting}
							{sectionCreatedHandler}
							{skill}
						/>
					{/each}
				</ul>
			</section>
		{:else if section.sectionType === ESectionTypes.Language}
			<SectionTitle {designFont} title={section.sectionTitle} {titleCreatedHandler} />
			<div class="flex flex-row flex-wrap items-start gap-x-4 gap-y-4">
				{#each section.data.rows as language, langIdx ('language-' + language.id)}
					<Language
						{designFont}
						displaySetting={section.data.displaySetting}
						{language}
						{sectionCreatedHandler}
					/>
				{/each}
			</div>
		{/if}
	{/each}
</div>
