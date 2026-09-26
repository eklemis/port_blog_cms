<script lang="ts">
	import { untrack } from 'svelte';
	import { Button, EmptyState, Field, InlineAlert } from '$lib/shared/ui';
	import { CONSOLE_ROUTES } from '$lib/shared/config/routes';
	import {
		CollectionCard,
		ExperienceList,
		patchCv,
		replaceExperiences
	} from '$lib/features/edit-cv';
	import {
		CONTACT_TYPES,
		contactTypeLabel,
		type ContactDetail,
		type CoreSkill,
		type Education,
		type Experience
	} from '$lib/entities/cv';
	import type { HandlingClass } from '$lib/shared/lib/error-class';

	/**
	 * `/studio/resumes/[id]` — Screen / CV builder 69:2.
	 *
	 * §02 calls it "the heaviest form in the product: five repeatable collections
	 * in one document". This pass builds the two the frame draws in full —
	 * identity, experience, and the three short collections. Highlighted projects
	 * is the one still showing a count alone: its DTO carries an `id` and a
	 * `slug`, so it picks from the author's own projects rather than being typed,
	 * and a picker needs the projects list the loader does not fetch yet.
	 *
	 * **No frame shows a collection open.** 69:2 draws the four as a name, a
	 * count and "+ Add", so the open state is borrowed from the experience list
	 * on the same screen rather than invented.
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
		core_skills: CoreSkill[];
		educations: Education[];
		highlighted_projects: unknown[];
		contact_info: ContactDetail[];
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
	let skills = $state<CoreSkill[]>(untrack(() => cv?.core_skills ?? []));
	let educations = $state<Education[]>(untrack(() => cv?.educations ?? []));
	let contacts = $state<ContactDetail[]>(untrack(() => cv?.contact_info ?? []));

	let saving = $state(false);
	let saved = $state(false);
	let failure = $state<string | undefined>();
	let failureKind = $state<HandlingClass>('notOurs');

	const highlighted = $derived(cv?.highlighted_projects.length ?? 0);

	async function save() {
		if (!cv || saving) return;

		saving = true;
		failure = undefined;

		const result = await patchCv(
			cv.id,
			{
				role: role.trim(),
				display_name: displayName.trim(),
				experiences: replaceExperiences(experiences),
				core_skills: { replace: skills },
				educations: { replace: educations },
				contact_info: { replace: contacts }
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
				<CollectionCard
					label="Core skills"
					items={skills}
					blank={() => ({ title: '', description: '' })}
					summary={(skill) => skill.title || 'New skill'}
					onchange={(next) => (skills = next)}
				>
					{#snippet fields(skill, update)}
						<Field
							id="skill-title"
							label="Title"
							value={skill.title}
							oninput={(event) =>
								update({ title: (event.currentTarget as HTMLInputElement).value })}
						/>
						<Field
							id="skill-description"
							label="Description"
							value={skill.description}
							oninput={(event) =>
								update({ description: (event.currentTarget as HTMLInputElement).value })}
						/>
					{/snippet}
				</CollectionCard>

				<CollectionCard
					label="Education"
					items={educations}
					blank={() => ({ institution: '', degree: '', graduation_year: 0 })}
					summary={(entry) => entry.institution || 'New entry'}
					onchange={(next) => (educations = next)}
				>
					{#snippet fields(entry, update)}
						<Field
							id="education-institution"
							label="Institution"
							value={entry.institution}
							oninput={(event) =>
								update({ institution: (event.currentTarget as HTMLInputElement).value })}
						/>
						<Field
							id="education-degree"
							label="Degree"
							value={entry.degree}
							oninput={(event) =>
								update({ degree: (event.currentTarget as HTMLInputElement).value })}
						/>
						<Field
							id="education-year"
							label="Graduation year"
							value={entry.graduation_year ? String(entry.graduation_year) : ''}
							oninput={(event) =>
								update({
									graduation_year: Number((event.currentTarget as HTMLInputElement).value) || 0
								})}
						/>
					{/snippet}
				</CollectionCard>

				<!-- Highlighted projects picks from the author's own projects — its DTO
				     carries an id and a slug — so it waits on the projects list the
				     loader does not fetch yet. -->
				<section
					aria-label="Highlighted projects"
					class="flex flex-col gap-2 rounded-xl border border-arch-line bg-arch-surface p-5"
				>
					<h2 class="font-mono text-[9px] font-normal tracking-[0.9px] text-arch-muted uppercase">
						Highlighted projects
					</h2>
					<p class="text-[12px] text-arch-muted">
						{highlighted}
						{highlighted === 1 ? 'entry' : 'entries'}
					</p>
				</section>

				<CollectionCard
					label="Contact details"
					items={contacts}
					blank={() => ({ contact_type: 'web_page' as const, title: '', content: '' })}
					summary={(row) => row.title || contactTypeLabel(row.contact_type)}
					onchange={(next) => (contacts = next)}
				>
					{#snippet fields(row, update)}
						<label class="flex flex-col gap-[5px]">
							<span class="text-[11.5px] text-arch-muted">Kind</span>
							<select
								value={row.contact_type}
								onchange={(event) =>
									update({
										contact_type: event.currentTarget.value as ContactDetail['contact_type']
									})}
								class="h-[34px] rounded-[7px] border border-arch-line-control bg-arch-surface px-3
								       text-[12.5px] text-arch-headline"
							>
								{#each CONTACT_TYPES as type (type)}
									<option value={type}>{contactTypeLabel(type)}</option>
								{/each}
							</select>
						</label>
						<Field
							id="contact-title"
							label="Title"
							value={row.title}
							oninput={(event) =>
								update({ title: (event.currentTarget as HTMLInputElement).value })}
						/>
						<Field
							id="contact-content"
							label="Content"
							value={row.content}
							oninput={(event) =>
								update({ content: (event.currentTarget as HTMLInputElement).value })}
						/>
					{/snippet}
				</CollectionCard>
			</div>
		</div>
	</div>
{/if}
