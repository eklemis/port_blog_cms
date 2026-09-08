import { FileText, Folder, Image, LayoutDashboard, Tag, User } from '@lucide/svelte';
import type { Component } from 'svelte';
import { CONSOLE_ROUTES } from '$lib/shared/config/routes';

/**
 * The console's navigation, defined once.
 *
 * The sidebar, the tablet rail and the mobile tab bar all read this, so they
 * cannot drift from one another — the blueprint's point that "the fifth screen
 * someone opens feels like the first four" starts here.
 *
 * Icons are the Lucide glyphs the Figma components name.
 */

export type NavItem = {
	label: string;
	href: string;
	icon: Component;
};

export const NAV: NavItem[] = [
	{ label: 'Overview', href: CONSOLE_ROUTES.overview, icon: LayoutDashboard },
	{ label: 'Posts', href: CONSOLE_ROUTES.posts, icon: FileText },
	{ label: 'Projects', href: CONSOLE_ROUTES.projects, icon: Folder },
	{ label: 'Résumés', href: CONSOLE_ROUTES.resumes, icon: User },
	{ label: 'Media', href: CONSOLE_ROUTES.media, icon: Image },
	{ label: 'Topics', href: CONSOLE_ROUTES.topics, icon: Tag }
];

/** Reached below the fold of the nav, after a rule. */
export const ACCOUNT: NavItem = {
	label: 'Account',
	href: CONSOLE_ROUTES.account,
	icon: User
};

/**
 * Five at 390px, and everything else behind More — a wrapped tab row pushes the
 * first result below the fold, which defeats the point of the screen.
 */
export const TAB_BAR: NavItem[] = [
	NAV[1], // Posts
	NAV[2], // Projects
	NAV[3], // Résumés
	NAV[4], // Media
	{ label: 'More', href: CONSOLE_ROUTES.account, icon: Tag }
];

/**
 * Whether `path` is inside `href`'s section.
 *
 * Not `startsWith`: `/studio` is a prefix of every console path, so that would
 * leave Overview lit on all of them, and `/studio/post` would light `/studio/posts`.
 */
/**
 * What to call the screen at `path`.
 *
 * Derived from the same list the nav is, so the mobile bar cannot name one
 * screen while the tab bar lights another — which is exactly what it did when
 * each page was left to declare its own title.
 */
export function currentLabel(path: string): string {
	const match = [...NAV, ACCOUNT].find((item) => isCurrent(item.href, path));
	return match?.label ?? 'Console';
}

export function isCurrent(href: string, path: string): boolean {
	const here = path.replace(/\/+$/, '') || '/';
	const section = href.replace(/\/+$/, '');

	if (section === CONSOLE_ROUTES.overview) return here === section;

	return here === section || here.startsWith(`${section}/`);
}
