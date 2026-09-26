import type { PageServerLoad } from './$types';
import { authenticatedFetch } from '$lib/shared/api/backend.server';
import type { ContactDetail, CoreSkill, Education, Experience } from '$lib/entities/cv';

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
	/** Still `unknown` here: nothing renders it until the picker exists. */
	highlighted_projects: unknown[];
	contact_info: ContactDetail[];
};

export const load: PageServerLoad = async (event) => {
	const id = encodeURIComponent(event.params.id ?? '');

	try {
		const response = await authenticatedFetch(event, `/api/cvs/${id}`);
		if (!response.ok) return { cv: null, denied: true };

		const body = (await response.json()) as { data?: Cv };
		if (!body.data?.id) return { cv: null, denied: true };

		return { cv: body.data, denied: false };
	} catch {
		// Unreachable is indistinguishable from refused here, and the refusal is
		// the safer thing to show: it offers a way back.
		return { cv: null, denied: true };
	}
};
