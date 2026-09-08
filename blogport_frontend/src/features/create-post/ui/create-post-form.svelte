<script lang="ts">
	import { tick } from 'svelte';
	import { Button, Field, InlineAlert } from '$lib/shared/ui';
	import type { HandlingClass } from '$lib/shared/lib/error-class';
	import { SLUG_MAX, slugError, slugFrom } from '$lib/shared/lib/slug';
	import { TITLE_COUNTER_FROM, TITLE_MAX, contentError, titleError } from '$lib/entities/post';
	import { checkSlug, createPost, type CreateField } from '../api/create-post';

	/**
	 * "New post" — J4 step one.
	 *
	 * The smallest thing that can exist, because the post has to exist before it
	 * can have a cover image: media attaches to a `target_id`. Everything after
	 * the first save belongs to the editor.
	 *
	 * The body is required here, which the journey does not say and the API
	 * does: `validate_content` refuses an empty body outright, so a genuinely
	 * blank draft cannot be created. Reported rather than worked around.
	 */
	let {
		oncreated,
		/** Injected by the spec; the browser's own otherwise. */
		fetchFn = undefined
	}: {
		oncreated: (id: string) => void;
		fetchFn?: typeof globalThis.fetch;
	} = $props();

	let title = $state('');
	let slug = $state('');
	let content = $state('');

	/** Until someone writes their own, the address follows the title. */
	let slugIsTheirs = $state(false);

	let titleProblem = $state<string | undefined>();
	let slugProblem = $state<string | undefined>();
	let contentProblem = $state<string | undefined>();

	let failure = $state<string | undefined>();
	/** Which of §07's six it was, so the colour is the class's and not a guess. */
	let failureKind = $state<HandlingClass>('notOurs');

	let suggestion = $state<string | null>(null);
	let submitting = $state(false);
	let check: ReturnType<typeof setTimeout> | undefined;

	/**
	 * Shown only once the request has been slow enough to be worth reporting.
	 * Forms Spec §05 keeps a 400ms floor: a spinner that appears and vanishes
	 * inside a blink reads as a glitch rather than as progress.
	 */
	let slow = $state(false);
	let spinner: ReturnType<typeof setTimeout> | undefined;

	$effect(() => () => {
		clearTimeout(check);
		clearTimeout(spinner);
	});

	/**
	 * Ask whether the address is free, on a pause.
	 *
	 * A courtesy ahead of the real answer, which is `SLUG_ALREADY_EXISTS` on
	 * create — so it never blocks a submit, it only offers the free variant the
	 * backend found. Debounced for the same reason the search box is: a request
	 * per keystroke is a rate limit waiting to happen.
	 */
	function askAboutSlug(candidate: string) {
		clearTimeout(check);
		suggestion = null;
		if (!candidate) return;

		check = setTimeout(async () => {
			const answer = await checkSlug(candidate, fetchFn);
			// Ignore an answer about an address that has since been retyped.
			if (candidate !== slug) return;

			suggestion = answer.available ? null : answer.suggestion;
		}, 300);
	}

	// Field writes the bound value before it calls back, so both of these read
	// state rather than picking the value off an untyped Event.
	function typedTitle() {
		if (titleProblem) titleProblem = titleError(title);
		if (slugIsTheirs) return;

		slug = slugFrom(title);
		slugProblem = undefined;
		askAboutSlug(slug);
	}

	function typedSlug() {
		// Retyping an address that keeps being replaced is the worst kind of
		// fight to have with a form.
		slugIsTheirs = true;
		if (slugProblem) slugProblem = slugError(slug);
		askAboutSlug(slug.trim());
	}

	function useSuggestion() {
		if (!suggestion) return;

		slug = suggestion;
		slugIsTheirs = true;
		slugProblem = undefined;
		suggestion = null;
	}

	async function focusFirstInvalid() {
		await tick();
		const id = titleProblem
			? 'new-post-title'
			: slugProblem
				? 'new-post-slug'
				: contentProblem
					? 'new-post-content'
					: null;
		if (!id) return;

		const field = document.getElementById(id);
		field?.focus();
		field?.scrollIntoView({ block: 'nearest' });
	}

	const under: Record<CreateField, (message: string) => void> = {
		title: (message) => (titleProblem = message),
		slug: (message) => (slugProblem = message),
		content: (message) => (contentProblem = message)
	};

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		if (submitting) return;

		// Submit validates everything, whether or not it was touched.
		titleProblem = titleError(title);
		slugProblem = slugError(slug);
		contentProblem = contentError(content);

		if (titleProblem || slugProblem || contentProblem) {
			failure = undefined;
			await focusFirstInvalid();
			return;
		}

		submitting = true;
		failure = undefined;
		spinner = setTimeout(() => (slow = true), 400);

		const result = await createPost({ title: title.trim(), slug: slug.trim(), content }, fetchFn);

		submitting = false;
		clearTimeout(spinner);
		slow = false;

		if (result.ok) {
			oncreated(result.id);
			return;
		}

		// Every value stays: losing a written draft to one refused field is the
		// worst outcome available.
		if (result.field) {
			under[result.field](result.message);
			await focusFirstInvalid();
			return;
		}

		failure = result.message;
		failureKind = result.kind;
	}

	const titleCount = $derived([...title].length);
	const showCounter = $derived(titleCount >= TITLE_COUNTER_FROM);
</script>

<form novalidate onsubmit={submit} class="flex w-full flex-col gap-[18px] md:gap-5">
	<Field
		id="new-post-title"
		label="Title"
		name="title"
		placeholder="Building a CMS in Rust"
		required
		bind:value={title}
		error={titleProblem}
		oninput={typedTitle}
		onblur={() => (titleProblem = titleError(title))}
	/>

	{#if showCounter}
		<!-- A count, not a refusal: the field still takes what is typed and the
		     server is what finally decides. -->
		<p class="-mt-3 text-[11.5px] {titleCount > TITLE_MAX ? 'text-st-danger' : 'text-arch-muted'}">
			{titleCount} / {TITLE_MAX}
		</p>
	{/if}

	<Field
		id="new-post-slug"
		label="Web address"
		name="slug"
		maxlength={SLUG_MAX}
		placeholder="building-a-cms-in-rust"
		help="This becomes the post's public address."
		required
		bind:value={slug}
		error={slugProblem}
		oninput={typedSlug}
		onblur={() => (slugProblem = slugError(slug))}
	/>

	{#if suggestion}
		<!-- The backend found this one and says it is free, which a "-2" guessed
		     here would not be. -->
		<div class="-mt-3 flex items-center gap-2.5">
			<p class="text-[11.5px] text-st-inflight">That address is taken.</p>
			<Button kind="ghost" label="Use {suggestion}" onclick={useSuggestion} />
		</div>
	{/if}

	<div class="flex flex-col gap-1.5">
		<label for="new-post-content" class="text-[12.5px] font-medium text-arch-headline">
			Post
		</label>
		<!-- Not a Field: that component is a single-line input by specification,
		     and the body is the one thing on this screen that is not. -->
		<textarea
			id="new-post-content"
			name="content"
			rows="8"
			placeholder="Write the first line. You can finish it in the editor."
			bind:value={content}
			aria-invalid={contentProblem ? 'true' : undefined}
			aria-describedby={contentProblem ? 'new-post-content-error' : undefined}
			class="w-full rounded-lg border bg-arch-surface px-3 py-2.5 text-[13px]
			       text-arch-headline placeholder:text-arch-muted
			       {contentProblem ? 'border-st-danger' : 'border-arch-line-control'}"
			oninput={() => contentProblem && (contentProblem = contentError(content))}
			onblur={() => (contentProblem = contentError(content))}
		></textarea>
		{#if contentProblem}
			<p id="new-post-content-error" class="text-[11.5px] text-st-danger">{contentProblem}</p>
		{/if}
	</div>

	<InlineAlert message={failure} kind={failureKind} />

	<div class="flex">
		<Button type="submit" label="Create draft" loading={slow} disabled={submitting} />
	</div>
</form>
