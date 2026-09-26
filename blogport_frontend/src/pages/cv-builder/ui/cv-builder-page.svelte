<script lang="ts">
	import { untrack } from 'svelte';
	import { Button, EmptyState, Field, InlineAlert } from '$lib/shared/ui';
	import { CONSOLE_ROUTES } from '$lib/shared/config/routes';
	import { ExperienceList, patchCv, replaceExperiences } from '$lib/features/edit-cv';
	import type { Experience } from '$lib/entities/cv';
	import type { HandlingClass } from '$lib/shared/lib/error-class';

	/**
	 * `/studio/resumes/[id]` — Screen / CV builder 69:2.
	 *
	 * §02 calls it "the heaviest form in the product: five repeatable collections
	 * in one document". This pass builds the two the frame draws in full —
	 * identity, and experience — and shows the other four as the frame shows
	 * them: a name and a count. Their editors are the next slice, so the "+ Add"
	 * the frame puts on each is not here; a control that opens nothing is worse
	 * than one that is missing.
	 *
	 * **"Written in English" is not built.** The frame draws a content-language
	 * pill beside the title — §06's second language setting, the one that belongs
	 * to the document rather than the reader. `PatchCVRequest` has no `language`
	 * field and neither does `CvResponse`; only a cover letter carries one. Filed
	 * rather than faked: a control that cannot save is a promise the page breaks.
	 */
	type Cv = {
		id: string;
		role: string;
		display_name: string;
		experiences: Experience[];
		core_skills: unknown[];
		educations: unknown[];
		highlighted_projects: unknown[];
		contact_info: unknown[];
	};

	let {
		cv,
		onsaved = () => {},
		fetchFn = undefined,
		denied = false
	}: {
		cv: Cv | null;
		onsaved?: () => void;
		fetchFn?: typeof globalThis.fetch;
		denied?: boolean;
	} = $props();

	// Seeds, not bindings: the document is the person's from the moment it opens.
	let role = $state(untrack(() => cv?.role ?? ''));
	let displayName = $state(untrack(() => cv?.display_name ?? ''));
	let experiences = $state<Experience[]>(untrack(() => cv?.experiences ?? []));

	let saving = $state(false);
	let saved = $state(false);
	let failure = $state<string | undefined>();
	let failureKind = $state<HandlingClass>('notOurs');

	const collections = $derived([
		{ label: 'Core skills', count: cv?.core_skills.length ?? 0 },
		{ label: 'Education', count: cv?.educations.length ?? 0 },
		{ label: 'Highlighted projects', count: cv?.highlighted_projects.length ?? 0 },
		{ label: 'Contact details', count: cv?.contact_info.length ?? 0 }
	]);

	async function save() {
		if (!cv || saving) return;

		saving = true;
		failure = undefined;

		const result = await patchCv(
			cv.id,
			{
				role: role.trim(),
				display_name: displayName.trim(),
				experiences: replaceExperiences(experiences)
			},
			fetchFn
		);

		saving = false;

		if (!result.ok) {
			failure = result.message;
			failureKind = result.kind;
			return;
		}

		saved = true;
		onsaved();
	}
</script>

{#if denied || !cv}
	<EmptyState
		title="We couldn’t find that résumé."
		message="It may belong to someone else, or it may have been deleted."
	>
		{#snippet action()}
			<Button kind="secondary" label="Back to résumés" href={CONSOLE_ROUTES.resumes} />
		{/snippet}
	</EmptyState>
{:else}
	<div class="flex flex-col gap-4">
		<div class="flex flex-wrap items-start justify-between gap-3">
			<div class="flex flex-col gap-0.5">
				<h1 class="font-display text-[19px] font-bold text-arch-headline">
					{role.trim() || 'Untitled résumé'}
				</h1>
				<p class="text-[11.5px] text-arch-muted">Résumé</p>
			</div>
			<div class="flex items-center gap-2">
				<Button label="Save" loading={saving} onclick={save} />
			</div>
		</div>

		<InlineAlert message={failure} kind={failureKind} />

		<p class="sr-only" aria-live="polite">{saved && !failure ? 'Résumé saved.' : ''}</p>

		<div class="flex flex-col gap-4 lg:flex-row lg:items-start">
			<div class="flex w-full flex-col gap-4 lg:max-w-[548px]">
				<section
					aria-label="Identity"
					class="flex flex-col gap-3.5 rounded-xl border border-arch-line bg-arch-surface p-5"
				>
					<h2 class="font-mono text-[9px] font-normal tracking-[0.9px] text-arch-muted uppercase">
						Identity
					</h2>
					<div class="grid grid-cols-1 gap-2.5 md:grid-cols-2">
						<Field id="cv-role" label="Role" bind:value={role} />
						<Field id="cv-display-name" label="Display name" bind:value={displayName} />
					</div>
				</section>

				<ExperienceList roles={experiences} onchange={(next) => (experiences = next)} />
			</div>

			<div class="flex w-full flex-col gap-3.5 lg:max-w-[292px]">
				{#each collections as collection (collection.label)}
					<section
						aria-label={collection.label}
						class="flex flex-col gap-2 rounded-xl border border-arch-line bg-arch-surface p-5"
					>
						<h2 class="font-mono text-[9px] font-normal tracking-[0.9px] text-arch-muted uppercase">
							{collection.label}
						</h2>
						<p class="text-[12px] text-arch-muted">
							{collection.count}
							{collection.count === 1 ? 'entry' : 'entries'}
						</p>
					</section>
				{/each}
			</div>
		</div>
	</div>
{/if}
