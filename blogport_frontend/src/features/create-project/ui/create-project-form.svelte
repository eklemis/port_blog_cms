<script lang="ts">
	import { Button, ChipInput, Field, InlineAlert } from '$lib/shared/ui';
	import { SLUG_MAX, slugError, slugFrom } from '$lib/shared/lib/slug';
	import { CONSOLE_ROUTES } from '$lib/shared/config/routes';
	import type { HandlingClass } from '$lib/shared/lib/error-class';
	import { SLUG_TAKEN, checkSlug, createProject, type CreateField } from '../api/create-project';

	/**
	 * Adding a project. No frame draws this screen; §02 and §03 specify it.
	 *
	 * **Pressing the button publishes.** §02: "Projects have no `published_at`.
	 * Creating one publishes it. Say so on the create button — 'Add project' with
	 * the helper 'Projects are visible on your public page as soon as they're
	 * added.'" So the helper sits under the button, before the press, not in a
	 * toast after it.
	 *
	 * Four fields, and the line is where the id matters: media attaches to a
	 * `target_id` and topics attach by id, so both need the project to exist
	 * first. Title, address, description and tech stack are all plain data on
	 * `CreateProjectRequest`, and the frame puts all four here — which answers
	 * the question I raised when this screen was undrawn.
	 *
	 * The address is checked as it is typed, which matters more here than on a
	 * post: `PatchProjectRequest` carries no slug, so this is the only moment a
	 * project's address can be got right.
	 */
	let {
		oncreated,
		fetchFn = undefined
	}: {
		/** The project exists and is live. The caller decides where to go. */
		oncreated: (id: string) => void;
		fetchFn?: typeof globalThis.fetch;
	} = $props();

	let title = $state('');
	let slug = $state('');
	let description = $state('');
	let stack = $state<string[]>([]);

	/** Once someone edits the address, the title stops writing it. */
	let slugTouched = $state(false);
	let suggestion = $state<string | null>(null);
	let errors = $state<Partial<Record<CreateField, string>>>({});
	let failure = $state<string | undefined>();
	let failureKind = $state<HandlingClass>('notOurs');
	let saving = $state(false);

	let timer: ReturnType<typeof setTimeout> | undefined;

	/**
	 * Ask whether the address is free, after a pause in typing.
	 *
	 * A courtesy ahead of the real answer on create — so a check that could not
	 * be made says nothing rather than blocking a free address.
	 */
	function askAbout(candidate: string) {
		clearTimeout(timer);
		suggestion = null;

		if (!candidate || slugError(candidate)) return;

		timer = setTimeout(async () => {
			const answer = await checkSlug(candidate, fetchFn);

			// The field may have moved on while the answer was in flight.
			if (candidate !== slug) return;

			suggestion = answer.available ? null : answer.suggestion;
			errors = answer.available ? { ...errors, slug: undefined } : { ...errors, slug: SLUG_TAKEN };
		}, 250);
	}

	function typedTitle(value: string) {
		title = value;
		errors = { ...errors, title: undefined };

		if (slugTouched) return;

		slug = slugFrom(value);
		askAbout(slug);
	}

	function typedSlug(value: string) {
		// Normalised as they type. The slug becomes the public URL, and a field
		// that silently changes its value after save is a small dishonesty.
		slugTouched = true;
		slug = slugFrom(value);
		errors = { ...errors, slug: undefined };
		askAbout(slug);
	}

	function take(free: string) {
		slugTouched = true;
		slug = free;
		suggestion = null;
		errors = { ...errors, slug: undefined };
	}

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		if (saving) return;

		// §05: submit validates everything and focuses the first thing wrong.
		const found: Partial<Record<CreateField, string>> = {};
		if (!title.trim()) found.title = 'A title is required.';
		if (!description.trim()) found.description = 'A description is required.';

		const address = slugError(slug);
		if (address) found.slug = address;

		errors = found;
		if (Object.keys(found).length) return;

		saving = true;
		failure = undefined;

		const result = await createProject(
			{
				title: title.trim(),
				slug,
				description: description.trim(),
				// Omitted when empty rather than sent as [], so the server's default
				// stands for a project whose stack nobody listed.
				...(stack.length ? { tech_stack: [...stack] } : {})
			},
			fetchFn
		);

		saving = false;

		if (!result.ok) {
			// Every field stays. Retyping a form is the punishment for the
			// server's bad day.
			if (result.field) errors = { ...errors, [result.field]: result.message };
			else {
				failure = result.message;
				failureKind = result.kind;
			}
			return;
		}

		oncreated(result.id);
	}
</script>

<form
	class="flex w-full max-w-[560px] flex-col gap-[18px] rounded-xl border border-arch-line
	       bg-arch-surface p-[22px]"
	onsubmit={submit}
	novalidate
>
	<InlineAlert message={failure} kind={failureKind} />

	<Field
		id="new-project-title"
		label="Title"
		required
		bind:value={title}
		error={errors.title}
		oninput={(event) => typedTitle((event.currentTarget as HTMLInputElement).value)}
	/>

	<div class="flex flex-col gap-1.5">
		<Field
			id="new-project-slug"
			label="Address"
			required
			maxlength={SLUG_MAX}
			bind:value={slug}
			error={errors.slug}
			help="This becomes the public address, and it cannot be changed later."
			oninput={(event) => typedSlug((event.currentTarget as HTMLInputElement).value)}
		/>

		{#if suggestion}
			<!-- Suggest, don't discard: what they typed stays in the field. -->
			<div>
				<Button
					kind="secondary"
					size="compact"
					label="Use {suggestion}"
					onclick={() => take(suggestion ?? '')}
				/>
			</div>
		{/if}
	</div>

	<div class="flex flex-col gap-1.5">
		<label for="new-project-description" class="text-[12px] text-arch-muted">Description</label>
		<textarea
			id="new-project-description"
			bind:value={description}
			rows="3"
			aria-invalid={errors.description ? 'true' : undefined}
			aria-describedby="new-project-description-help{errors.description
				? ' new-project-description-error'
				: ''}"
			class="h-[74px] resize-y rounded-lg border border-arch-line bg-arch-surface px-3.5 py-[11px]
			       text-[13px] leading-[20px] text-arch-headline"
		></textarea>
		{#if errors.description}
			<p id="new-project-description-error" class="text-[11px] text-st-danger">
				{errors.description}
			</p>
		{/if}
		<p id="new-project-description-help" class="text-[11px] text-arch-muted">
			Shown under the title on your public page. A project goes live the moment it is added.
		</p>
	</div>

	<ChipInput
		label="Tech stack"
		values={stack}
		help="A chip input. Press Enter after each one."
		onchange={(next) => (stack = next)}
	/>

	<div class="h-px w-full bg-arch-line" role="presentation"></div>

	<!-- The frame puts the warning beside the buttons rather than under them:
	     it is read in the same glance as the thing it warns about. -->
	<div class="flex flex-wrap items-center justify-between gap-3">
		<p class="max-w-[300px] text-[11.5px] text-arch-muted">
			Projects are visible on your public page as soon as they’re added.
		</p>
		<div class="flex items-center gap-2">
			<Button kind="ghost" label="Cancel" href={CONSOLE_ROUTES.projects} />
			<Button type="submit" label="Add project" loading={saving} />
		</div>
	</div>
</form>
