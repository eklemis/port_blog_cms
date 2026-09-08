<script lang="ts">
	import { greeting } from '../model/greeting';

	/**
	 * The console's front door.
	 *
	 * Three stat tiles, not the four the frame draws, and without the sub-lines
	 * beneath them: "Applications" is in no route the blueprint names, and
	 * "12 live · 9 drafts · 3 archived" needs per-status counts that a single
	 * `total` does not carry. "Needs attention" and "Getting started" want
	 * media and applications, which this route is not sanctioned to call.
	 * All of it is reported rather than guessed — see the PR.
	 *
	 * Design: Screen / Overview 68:2 · Mobile / Overview 90:2188.
	 */
	let {
		fullName,
		counts
	}: {
		fullName: string;
		counts: { posts: number | null; projects: number | null; resumes: number | null };
	} = $props();

	const tiles = $derived([
		{ label: 'Posts', value: counts.posts },
		{ label: 'Projects', value: counts.projects },
		{ label: 'Résumés', value: counts.resumes }
	]);
</script>

<div class="flex flex-col gap-3 md:gap-[18px]">
	<h1
		class="font-display text-[24px] font-extrabold tracking-tight text-arch-headline
		       md:text-[27px]"
	>
		{greeting(fullName)}
	</h1>

	<div class="flex flex-wrap gap-[11px] md:gap-[14px]">
		{#each tiles as tile (tile.label)}
			<div
				class="flex min-w-[160px] flex-1 flex-col gap-[5px] rounded-[11px] border
				       border-arch-line bg-arch-surface px-[15px] py-3.5 md:max-w-[215px]
				       md:gap-[7px] md:rounded-[12px] md:p-5"
			>
				<p class="font-display text-[24px] font-extrabold text-arch-headline md:text-[30px]">
					<!-- An em dash, not a nought: a count we could not fetch is not zero. -->
					{tile.value ?? '—'}
				</p>
				<p
					class="text-[11.5px] text-arch-muted md:text-[12.5px] md:font-medium
					       md:text-arch-headline"
				>
					{tile.label}
				</p>
			</div>
		{/each}
	</div>
</div>
