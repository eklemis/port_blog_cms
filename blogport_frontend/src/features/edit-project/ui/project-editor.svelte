<script lang="ts">
	import { untrack } from 'svelte';
	import { X } from '@lucide/svelte';
	import { Button, Field, InlineAlert } from '$lib/shared/ui';
	import { TopicPicker, createTopic, type Topic } from '$lib/entities/topic';
	import type { HandlingClass } from '$lib/shared/lib/error-class';
	import { attachTopic, detachTopic, patchProject, type ProjectChanges } from '../api/project';
	import ScreenshotsCard from './screenshots-card.svelte';

	/**
	 * `/studio/projects/[id]` — Screen / Project editor 70:188.
	 *
	 * Details on the left, screenshots and topics on the right. Unlike the post
	 * editor this one has an explicit Save: a project has no draft state, so
	 * every keystroke autosaved would be a keystroke published.
	 *
	 * Field-level throughout — §02: "there is no full-replace PUT, so the editor
	 * is field-level and never sends an object it did not load first." Only what
	 * changed goes, and an emptied URL goes as `null`, since the endpoint says
	 * `null` clears and an empty string would be stored and later rendered as a
	 * link to nowhere.
	 *
	 * **The address is a fact here, not a field.** The frame draws it as an input
	 * identical to Title, but `CreateProjectRequest` takes a `slug` and
	 * `PatchProjectRequest` does not: a project's address is fixed when it is
	 * created. §04 settles the rendering — "Read-only facts render as text, not
	 * disabled inputs" — and the conflict is reported rather than papered over.
	 * It matters more than it looks: projects also have no restore, so a typo in
	 * an address cannot be fixed by deleting and starting again.
	 */
	type Project = {
		id: string;
		title: string;
		slug: string;
		description?: string | null;
		tech_stack: string[];
		topics: Topic[];
		repo_url?: string | null;
		live_demo_url?: string | null;
	};

	let {
		project,
		username,
		screenshots = [],
		availableTopics = [],
		onchanged = () => {},
		fetchFn = undefined
	}: {
		project: Project;
		/** Whose project it is. The public address is built from it. */
		username: string;
		screenshots?: { media_id: string; original_filename: string; src?: string | null }[];
		availableTopics?: Topic[];
		onchanged?: () => void;
		fetchFn?: typeof globalThis.fetch;
	} = $props();

	// Seeds, not bindings: the form is the person's from the moment it opens,
	// and a loader that re-ran would throw away whatever they had typed.
	let title = $state(untrack(() => project.title));
	let description = $state(untrack(() => project.description ?? ''));
	let repo = $state(untrack(() => project.repo_url ?? ''));
	let demo = $state(untrack(() => project.live_demo_url ?? ''));
	let stack = $state<string[]>(untrack(() => [...project.tech_stack]));
	let topics = $state<Topic[]>(untrack(() => [...project.topics]));

	let chip = $state('');
	let saving = $state(false);
	let failure = $state<string | undefined>();
	let failureKind = $state<HandlingClass>('notOurs');
	let saved = $state(false);

	const publicPath = $derived(
		`/${encodeURIComponent(username)}/projects/${encodeURIComponent(project.slug)}`
	);

	/** `null` clears; an unchanged field is not sent at all. */
	function urlChange(next: string, was: string | null | undefined) {
		const trimmed = next.trim();
		const before = was ?? '';

		if (trimmed === before) return undefined;

		return trimmed === '' ? null : trimmed;
	}

	const changes = $derived.by((): ProjectChanges => {
		const next: ProjectChanges = {};

		if (title.trim() !== project.title) next.title = title.trim();
		if (description.trim() !== (project.description ?? '')) {
			next.description = description.trim() || null;
		}

		const repoChange = urlChange(repo, project.repo_url);
		if (repoChange !== undefined) next.repo_url = repoChange;

		const demoChange = urlChange(demo, project.live_demo_url);
		if (demoChange !== undefined) next.live_demo_url = demoChange;

		// Compared element by element rather than by joining on a sentinel: any
		// separator is a character somebody could type, and ["Actix Web"] must
		// never read as ["Actix", "Web"].
		const sameStack =
			stack.length === project.tech_stack.length &&
			stack.every((tech, index) => tech === project.tech_stack[index]);

		if (!sameStack) next.tech_stack = [...stack];

		return next;
	});

	/** §03: "Enter or `,` commits; Backspace on an empty input removes the last." */
	function commit() {
		const value = chip.trim().replace(/,$/, '').trim();
		chip = '';

		if (!value || stack.includes(value)) return;

		stack = [...stack, value];
	}

	function typedChip(value: string) {
		chip = value;

		// A comma is how people type a list; committing on it means nobody has to
		// discover that Enter is the only way.
		if (value.includes(',')) commit();
	}

	async function save() {
		if (!Object.keys(changes).length || saving) return;

		saving = true;
		failure = undefined;

		const result = await patchProject(project.id, changes, fetchFn);

		saving = false;

		if (!result.ok) {
			// Every edit stays. A save that failed is not a reason to lose work.
			failure = result.message;
			failureKind = result.kind;
			return;
		}

		saved = true;
		onchanged();
	}

	const offerable = $derived(
		availableTopics.filter((option) => !topics.some((have) => have.id === option.id))
	);

	async function attach(topic: Topic) {
		topics = [...topics, topic];

		const result = await attachTopic(project.id, topic.id, fetchFn);

		if (!result.ok) {
			topics = topics.filter((have) => have.id !== topic.id);
			failure = result.message;
			failureKind = result.kind;
		}
	}

	async function detach(topic: Topic) {
		const before = topics;
		topics = topics.filter((have) => have.id !== topic.id);

		const result = await detachTopic(project.id, topic.id, fetchFn);

		if (!result.ok) {
			topics = before;
			failure = result.message;
			failureKind = result.kind;
		}
	}

	async function create(title: string) {
		const result = await createTopic(title, fetchFn);

		if (!result.ok) {
			failure = result.message;
			failureKind = result.kind;
			return;
		}

		await attach(result.topic);
	}
</script>

<!-- eslint-disable svelte/no-navigation-without-resolve --
	The public path is built from a username and a slug that arrive at runtime,
	already escaped; `resolve()` would encode them twice. -->

<div class="flex flex-col gap-4">
	<div class="flex flex-wrap items-center justify-between gap-3">
		<h1 class="font-display text-[19px] font-bold text-arch-headline">{project.title}</h1>
		<div class="flex items-center gap-2">
			<Button kind="ghost" label="View public page" href={publicPath} />
			<Button label="Save" loading={saving} onclick={save} />
		</div>
	</div>

	<InlineAlert message={failure} kind={failureKind} />

	<!-- One live region for the save, beside the action that causes it. -->
	<p class="sr-only" aria-live="polite">{saved && !failure ? 'Project saved.' : ''}</p>

	<div class="flex flex-col gap-4 lg:flex-row lg:items-start">
		<div
			class="flex w-full flex-col gap-3.5 rounded-xl border border-arch-line bg-arch-surface p-5
			       lg:max-w-[560px]"
		>
			<Field id="project-title" label="Title" bind:value={title} />

			<!-- Not a field: a project's address is fixed at creation, because
			     `PatchProjectRequest` carries no slug. -->
			<div class="flex flex-col gap-[5px]">
				<span class="text-[11.5px] text-arch-muted">Address</span>
				<p class="font-mono text-[12.5px] text-arch-headline">{project.slug}</p>
				<p class="text-[10.5px] text-arch-muted">
					Set when the project was created, and not changeable afterwards.
				</p>
			</div>

			<!-- A textarea rather than a Field: 70:227 draws it two lines deep, and
			     a description that wraps is the ordinary case. -->
			<div class="flex flex-col gap-[5px]">
				<label for="project-description" class="text-[11.5px] text-arch-muted">Description</label>
				<textarea
					id="project-description"
					bind:value={description}
					rows="3"
					class="resize-y rounded-[7px] border border-arch-line-control bg-arch-surface px-3 py-2.5
					       text-[12.5px] leading-[19px] text-arch-headline"
				></textarea>
			</div>

			<div class="flex flex-col gap-1.5" role="group" aria-labelledby="project-stack-label">
				<span id="project-stack-label" class="text-[11.5px] text-arch-muted">Tech stack</span>
				<div class="flex flex-wrap items-center gap-[7px]">
					{#each stack as tech (tech)}
						<span
							class="flex items-center gap-1.5 rounded-full bg-arch-surface-2 px-2.5 py-[5px]
							       text-[11px] text-arch-headline"
						>
							{tech}
							<button
								type="button"
								aria-label="Remove {tech}"
								onclick={() => (stack = stack.filter((have) => have !== tech))}
								class="text-arch-muted hover:text-arch-headline"
							>
								<X size={11} aria-hidden="true" />
							</button>
						</span>
					{/each}

					<input
						type="text"
						value={chip}
						aria-label="Add to tech stack"
						placeholder="Add…"
						oninput={(event) => typedChip(event.currentTarget.value)}
						onkeydown={(event) => {
							if (event.key === 'Enter') {
								event.preventDefault();
								commit();
							}
							if (event.key === 'Backspace' && chip === '') stack = stack.slice(0, -1);
						}}
						class="w-[90px] rounded-full border border-arch-line-control px-2.5 py-[5px]
						       text-[11px] text-arch-headline placeholder:text-arch-muted"
					/>
				</div>
				<p class="text-[10.5px] text-arch-muted">
					A chip input, never a comma-separated text field — nobody should have to guess the
					delimiter.
				</p>
			</div>

			<div class="flex flex-col gap-2.5 md:flex-row md:gap-2.5">
				<div class="flex-1">
					<Field id="project-repo" label="Repository" bind:value={repo} />
				</div>
				<div class="flex-1">
					<Field id="project-demo" label="Live demo" bind:value={demo} />
				</div>
			</div>
		</div>

		<div class="flex w-full flex-col gap-3.5 lg:max-w-[332px]">
			<ScreenshotsCard {screenshots} {fetchFn} onchanged={() => onchanged()} />

			<TopicPicker
				attached={topics}
				available={offerable}
				onattach={attach}
				ondetach={detach}
				oncreate={create}
			/>
		</div>
	</div>
</div>
