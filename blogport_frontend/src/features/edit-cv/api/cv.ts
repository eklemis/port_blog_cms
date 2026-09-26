import { UNEXPECTED } from '$lib/shared/lib/api-failure';
import { handlingClass, type HandlingClass } from '$lib/shared/lib/error-class';
import type { Experience } from '$lib/entities/cv';
import type { components } from '$lib/shared/api/v1';

/**
 * Saving a résumé.
 *
 * `PATCH /api/cvs/{id}` is field-level, and every collection on it is a
 * `ReplaceOp`: "The full replacement list. There is no per-item patch: a list
 * is replaced wholesale or left alone."
 *
 * **That wholesale replacement is the trap.** The builder draws company,
 * position, location, dates and tasks; `ExperienceDto` also requires
 * `achievements` and `description`. Rebuilding rows from the drawn fields alone
 * would delete work nobody touched — so a row goes back the way it arrived,
 * with only what was edited changed.
 */

type Fetch = typeof globalThis.fetch;

const mine: Fetch = (...args) => globalThis.fetch(...args);

export type CvChanges = components['schemas']['PatchCVRequest'];

export type SaveResult = { ok: true } | { ok: false; message: string; kind: HandlingClass };

/**
 * Wrap a list in the op the endpoint expects, and tidy what the fields leave
 * behind.
 *
 * `end_date` is dropped rather than sent empty — it is documented as "Absent
 * for a current position", so an empty string would read as a blank end date
 * rather than a job still running. Blank tasks go the same way: a row someone
 * added and never filled is not a responsibility.
 */
export function replaceExperiences(roles: Experience[]): CvChanges['experiences'] {
	return {
		replace: roles.map((role) => {
			const { end_date, ...rest } = role;

			return {
				...rest,
				tasks: role.tasks.filter((task) => task.trim()),
				achievements: role.achievements.filter((line) => line.trim()),
				...(end_date?.trim() ? { end_date } : {})
			};
		})
	};
}

export async function patchCv(
	cvId: string,
	changes: CvChanges,
	fetchFn: Fetch = mine
): Promise<SaveResult> {
	let response: Response;

	try {
		response = await fetchFn(`/api/cvs/${encodeURIComponent(cvId)}`, {
			method: 'PATCH',
			headers: { 'content-type': 'application/json' },
			body: JSON.stringify(changes)
		});
	} catch {
		return { ok: false, message: UNEXPECTED, kind: 'notOurs' };
	}

	if (!response.ok) {
		const body = (await response.json().catch(() => null)) as { error?: { code?: string } } | null;

		return { ok: false, message: UNEXPECTED, kind: handlingClass(body?.error?.code) };
	}

	return { ok: true };
}
