<script lang="ts">
	import { Check, Circle, X } from '@lucide/svelte';
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
		counts: Counts & { projects: number | null; applications: number | null };
	} = $props();

	const tiles = $derived([
		{ label: 'Posts', value: counts.posts },
		{ label: 'Projects', value: counts.projects },
		{ label: 'Résumés', value: counts.resumes },
		{ label: 'Applications', value: counts.applications }
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

	{#if firstRun}
		<!-- eslint-disable svelte/no-navigation-without-resolve --
			The three destinations come from shared/config/routes and two of them are
			console screens that do not exist yet — `resolve()` only takes a route id
			that does. Swap them in as each lands. -->
		<section
			aria-labelledby="getting-started"
			class="flex flex-col gap-3 rounded-[11px] border border-arch-line bg-arch-surface
			       px-[15px] py-3.5 md:max-w-[440px] md:rounded-[12px] md:p-5"
		>
			<div class="flex items-start justify-between gap-4">
				<div class="flex flex-col gap-1">
					<h2
						id="getting-started"
						class="font-display text-[14px] font-extrabold text-arch-headline"
					>
						Getting started
					</h2>
					<p class="text-[12px] text-arch-muted">Three things, then this goes away.</p>
				</div>
				<button
					type="button"
					aria-label="Dismiss"
					onclick={dismiss}
					class="-m-1 flex size-9 items-center justify-center rounded-lg text-arch-muted
					       transition-colors hover:text-arch-headline"
				>
					<X size={16} aria-hidden="true" />
				</button>
			</div>

			<ul class="flex flex-col gap-2.5">
				{#each tasks as task (task.label)}
					<li class="flex items-center gap-2.5">
						{#if task.done}
							<Check size={15} aria-hidden="true" class="shrink-0 text-st-live" />
						{:else}
							<Circle size={15} aria-hidden="true" class="shrink-0 text-arch-muted" />
						{/if}
						<a
							href={task.href}
							data-done={String(task.done === true)}
							class="text-[13px] font-medium text-arch-headline underline-offset-4 hover:underline"
						>
							{task.label}
						</a>
					</li>
				{/each}
			</ul>
		</section>
	{/if}
</div>
