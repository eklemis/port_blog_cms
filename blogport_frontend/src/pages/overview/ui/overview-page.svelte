<script lang="ts">
	import { X } from '@lucide/svelte';
	import { greeting } from '../model/greeting';
	import { gettingStarted, showGettingStarted, type Counts } from '../model/getting-started';

	/**
	 * The console's front door.
	 *
	 * Four stat tiles and the first-run checklist. Without the sub-lines the
	 * frame draws beneath the numbers, and without "Needs attention": both want
	 * per-status counts that `total` does not carry, and blueprint §09 says so
	 * itself — archived is unobtainable at all, and "applications with no reply"
	 * cannot be asked for because `GET /api/applications` takes no parameters.
	 *
	 * Design: Screen / Overview 68:2 · Mobile / Overview 90:2188.
	 */
	let {
		fullName,
		counts
	}: {
		fullName: string;
		/** `topics` backs the checklist rather than a tile — there are four tiles. */
		counts: Counts & {
			projects: number | null;
			applications: number | null;
			/** From /api/blog/summary; `null` when it could not be had. */
			postStates?: { live: number; drafts: number; archived: number } | null;
		};
	} = $props();

	const tiles = $derived([
		{
			label: 'Posts',
			value: counts.posts,
			// The only sub-line with a query behind it. The frame draws one on every
			// tile; the others wait for theirs rather than showing a guess.
			detail: counts.postStates
				? `${counts.postStates.live} live · ${counts.postStates.drafts} drafts · ${counts.postStates.archived} archived`
				: null
		},
		{ label: 'Projects', value: counts.projects, detail: null },
		{ label: 'Résumés', value: counts.resumes, detail: null },
		{ label: 'Applications', value: counts.applications, detail: null }
	]);

	/**
	 * Dismissal is this browser's business and nobody else's — there is no
	 * endpoint that remembers it, and a checklist that reappears on another
	 * device is a smaller problem than one that cannot be put away at all.
	 */
	const DISMISSED = 'arch:getting-started-dismissed';

	function wasDismissed() {
		try {
			return localStorage.getItem(DISMISSED) === 'yes';
		} catch {
			// Private mode, or storage the browser will not hand over. Showing the
			// checklist is the harmless half of being wrong.
			return false;
		}
	}

	let dismissed = $state(wasDismissed());

	function dismiss() {
		dismissed = true;
		try {
			localStorage.setItem(DISMISSED, 'yes');
		} catch {
			// It goes away for this visit either way.
		}
	}

	const tasks = $derived(gettingStarted(counts));
	const firstRun = $derived(showGettingStarted(tasks, dismissed));
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
				       border-arch-line bg-arch-surface px-[15px] py-3.5 md:w-[215px] md:max-w-[215px]
				       md:flex-none md:gap-[7px] md:rounded-[12px] md:p-5"
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
				{#if tile.detail}
					<!-- Desktop and tablet only: Mobile / Overview draws the tiles bare. -->
					<p class="text-[11px] text-arch-muted max-md:hidden">{tile.detail}</p>
				{/if}
			</div>
		{/each}
	</div>

	{#if firstRun}
		<!-- eslint-disable svelte/no-navigation-without-resolve --
			The three destinations come from shared/config/routes and two of them are
			console screens that do not exist yet — `resolve()` only takes a route id
			that does. Swap them in as each lands. -->
		<!-- Screen / Overview 68:62. Not on a phone: Mobile / Overview draws the
		     tiles and nothing else below them. -->
		<section
			aria-labelledby="getting-started"
			class="flex w-full max-w-[330px] flex-col gap-[11px] rounded-xl border border-arch-line
			       bg-arch-surface p-5 max-md:hidden"
		>
			<div class="flex items-center justify-between gap-4">
				<h2
					id="getting-started"
					class="font-mono text-[9px] font-normal tracking-[0.9px] text-arch-muted uppercase"
				>
					Getting started
				</h2>
				<!--
					Not drawn in the frame. §05 calls the checklist dismissible, and
					behaviour is the document's to decide — so it stays, as quietly as a
					control can be. Flagged in the PR.
				-->
				<button
					type="button"
					aria-label="Dismiss"
					onclick={dismiss}
					class="-my-2 -mr-2 flex size-8 items-center justify-center rounded-lg text-arch-muted
					       transition-colors hover:text-arch-headline"
				>
					<X size={14} aria-hidden="true" />
				</button>
			</div>

			<ul class="flex flex-col gap-[11px]">
				{#each tasks as task (task.label)}
					<li class="flex items-center gap-2.5">
						<span
							aria-hidden="true"
							class="size-4 shrink-0 rounded
							       {task.done ? 'bg-st-live' : 'border border-arch-line-strong'}"
						></span>
						<a
							href={task.href}
							data-done={String(task.done === true)}
							aria-label={task.done ? `Done: ${task.label}` : undefined}
							class="text-[12.5px] underline-offset-4 hover:underline
							       {task.done ? 'text-arch-muted' : 'text-arch-headline'}"
						>
							{task.label}
						</a>
					</li>
				{/each}
			</ul>
		</section>
	{/if}
</div>
