<script lang="ts">
	import { ProjectEditor } from '$lib/features/edit-project';
	import { Button, EmptyState } from '$lib/shared/ui';
	import { CONSOLE_ROUTES } from '$lib/shared/config/routes';
	import type { Topic } from '$lib/entities/topic';

	/**
	 * `/studio/projects/[id]`.
	 *
	 * Two shapes, like the post editor: the editor itself, and the page somebody
	 * reaches by pasting a URL for a project that is not theirs. A project
	 * belonging to someone else comes back as not found rather than forbidden, so
	 * both land here and neither bounces through login.
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
		fetchFn = undefined,
		denied = false
	}: {
		project: Project | null;
		username: string;
		screenshots?: { media_id: string; original_filename: string; src?: string | null }[];
		availableTopics?: Topic[];
		onchanged?: () => void;
		fetchFn?: typeof globalThis.fetch;
		denied?: boolean;
	} = $props();
</script>

{#if denied || !project}
	<EmptyState
		title="We couldn’t find that project."
		message="It may belong to someone else, or it may have been deleted."
	>
		{#snippet action()}
			<Button kind="secondary" label="Back to projects" href={CONSOLE_ROUTES.projects} />
		{/snippet}
	</EmptyState>
{:else}
	<ProjectEditor {project} {username} {screenshots} {availableTopics} {onchanged} {fetchFn} />
{/if}
