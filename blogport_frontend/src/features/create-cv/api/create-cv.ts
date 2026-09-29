import { UNEXPECTED } from '$lib/shared/lib/api-failure';
import { handlingClass, type HandlingClass } from '$lib/shared/lib/error-class';
import type { components } from '$lib/shared/api/v1';

/**
 * Making the empty résumé that "+ New résumé" opens.
 *
 * J6 starts here — "POST /api/cvs → 201 { id }" — and continues in the builder
 * with "Start from role and identity". There is no create-a-résumé screen in
 * the file, and inventing one would put role, name, bio and photo on a form
 * that then hands them to a builder drawing the same four fields again.
 *
 * So the document is born empty and named where it is written. The list falls
 * back to "Untitled résumé" for a row whose role is still blank, which is the
 * one moment that state is visible.
 *
 * **Every field goes, because every field is required.** `CreateCVRequest`
 * makes all nine mandatory — the five collections included — and `language` is
 * the only optional one. It is left off rather than sent as `en`: the field
 * documents itself as defaulting to `en`, and a client writing the default is a
 * client that has to be changed when the default does.
 */

type Fetch = typeof globalThis.fetch;

const mine: Fetch = (...args) => globalThis.fetch(...args);

export type CreateResult =
	| { ok: true; id: string }
	| { ok: false; message: string; kind: HandlingClass };

type NewCv = components['schemas']['CreateCVRequest'];

export async function createCv(displayName: string, fetchFn: Fetch = mine): Promise<CreateResult> {
	const body: NewCv = {
		bio: '',
		contact_info: [],
		core_skills: [],
		display_name: displayName,
		educations: [],
		experiences: [],
		highlighted_projects: [],
		photo_url: '',
		role: ''
	};

	let response: Response;

	try {
		response = await fetchFn('/api/cvs', {
			method: 'POST',
			headers: { 'content-type': 'application/json' },
			body: JSON.stringify(body)
		});
	} catch {
		return { ok: false, message: UNEXPECTED, kind: 'notOurs' };
	}

	const payload = (await response.json().catch(() => null)) as {
		data?: { id?: string };
		error?: { code?: string };
	} | null;

	if (!response.ok) {
		return { ok: false, message: UNEXPECTED, kind: handlingClass(payload?.error?.code) };
	}

	// A 201 carrying no id is not a created résumé anybody can open: navigating
	// on it produces a 404 that reads as the résumé being missing rather than
	// as the response being empty.
	const id = payload?.data?.id;
	if (!id) return { ok: false, message: UNEXPECTED, kind: 'notOurs' };

	return { ok: true, id };
}
