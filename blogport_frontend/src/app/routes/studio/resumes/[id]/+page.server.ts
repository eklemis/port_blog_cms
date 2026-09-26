import type { PageServerLoad } from './$types';
import { authenticatedFetch } from '$lib/shared/api/backend.server';
import type {
	ContactDetail,
	CoreSkill,
	Education,
	Experience,
	HighlightedProject
} from '$lib/entities/cv';

/**
 * `/studio/resumes/[id]` — the builder's document.
 *
 * A CV belonging to another author comes back as not found rather than
 * forbidden, so both outcomes are the same screen: a plain sentence and a route
 * back, never a bounce through login.
 */

type Cv = {
	id: string;
	role: string;
	display_name: string;
	experiences: Experience[];
	core_skills: CoreSkill[];
	educations: Education[];
	highlighted_projects: HighlightedProject[];
	contact_info: ContactDetail[];
};

type Project = { id: string; title: string; slug: string };

/**
 * The author's own projects, for the highlight picker.
 *
 * A highlighted row carries the project's `id` and `slug`, so the picker has to
 * offer real ones. Bounded by what one author owns and asked for in one call;
 * its failure costs the picker its options rather than the document.
 */
async function myProjects(event: Parameters<PageServerLoad>[0]): Promise<Project[]> {
	try {
		const response = await authenticatedFetch(event, '/api/projects?page=1&per_page=100');
		if (!response.ok) return [];

		const body = (await response.json()) as { data?: { items?: Project[] } };

		return (body.data?.items ?? []).filter((project) => project?.id);
	} catch {
		return [];
	}
}

export const load: PageServerLoad = async (event) => {
	const id = encodeURIComponent(event.params.id ?? '');

	try {
		const response = await authenticatedFetch(event, `/api/cvs/${id}`);
		if (!response.ok) return { cv: null, denied: true, projects: [] as Project[] };

		const body = (await response.json()) as { data?: Cv };
		if (!body.data?.id) return { cv: null, denied: true, projects: [] as Project[] };

		return { cv: body.data, denied: false, projects: await myProjects(event) };
	} catch {
		// Unreachable is indistinguishable from refused here, and the refusal is
		// the safer thing to show: it offers a way back.
		return { cv: null, denied: true, projects: [] as Project[] };
	}
};
