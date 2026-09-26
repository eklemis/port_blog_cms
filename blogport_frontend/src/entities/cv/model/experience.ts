import type { components } from '$lib/shared/api/v1';

/**
 * A role on a résumé — Screen / CV builder 69:2.
 *
 * The frame collapses each row to one line and says why: "Rows collapse to a
 * one-line summary. One opens at a time — ten expanded rows is a wall, not a
 * form."
 *
 * **The whole list is replaced or left alone.** `experiences` is a
 * `ReplaceOp`: "The full replacement list. There is no per-item patch." So an
 * editor that rebuilds rows from the fields it draws would drop every field it
 * does not — `achievements` and `description` are on the DTO and not on the
 * frame, and they must survive a save that never touched them.
 */

export type Experience = components['schemas']['ExperienceDto'];
export type CoreSkill = components['schemas']['CoreSkillDto'];
export type Education = components['schemas']['EducationDto'];
export type ContactDetail = components['schemas']['ContactDetailDto'];
export type HighlightedProject = components['schemas']['HighlightedProjectDto'];

/**
 * Which kinds of contact row exist: `phone_number` and `web_page`.
 *
 * **There is no email.** `ContactDetailDto.content` gives
 * `john@example.com` as its own example, and the designer's ruling on the
 * public profile was that an address belongs "on a résumé the author chose to
 * publish, where `contact_info` is part of a document they nominated" — so an
 * email on a CV is the intended case and the enum cannot express it. Raised;
 * until it changes, the two that exist are the two offered.
 */
export const CONTACT_TYPES = ['phone_number', 'web_page'] as const;

/** What a contact row is called on screen. */
export function contactTypeLabel(type: ContactDetail['contact_type']): string {
	return type === 'phone_number' ? 'Phone' : 'Web page';
}

/** What it was, then where. A row nobody has filled in is still findable. */
export function experienceSummary(role: Pick<Experience, 'company' | 'position'>): string {
	const parts = [role.position?.trim(), role.company?.trim()].filter(Boolean);

	return parts.length ? parts.join(' · ') : 'New role';
}

/**
 * `end_date` is documented as "Absent for a current position", so the frame's
 * "I work here now" is that absence rather than a flag of its own.
 */
export function isCurrent(role: Pick<Experience, 'end_date'>): boolean {
	return !role.end_date?.trim();
}

/** The year out of a free-form partial date: "2020-03" reads as "2020". */
function year(value: string | null | undefined): string {
	return value?.trim().slice(0, 4) ?? '';
}

/**
 * "2020 – 2022", or "2022 – now" for a job someone still has.
 *
 * The fields hold months because the API allows partial dates; the collapsed
 * summary is a glance, and months in a glance are noise.
 */
export function years(role: Pick<Experience, 'start_date' | 'end_date'>): string {
	const from = year(role.start_date);

	// Nothing to range from, so no dash to nowhere.
	if (!from) return '';

	return `${from} – ${isCurrent(role) ? 'now' : year(role.end_date)}`;
}
