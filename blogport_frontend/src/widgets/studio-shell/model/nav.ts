import {
	Briefcase,
	CircleUser,
	FileText,
	Folder,
	Image,
	LayoutDashboard,
	Tag,
	User
} from '@lucide/svelte';
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
	/**
	 * What the mobile bar prints when the full name will not fit a fifth of
	 * 390px. It is the visible text and therefore the accessible name too —
	 * an aria-label of "Applications" would not contain "Apps", which is the
	 * one thing 2.5.3 forbids.
	 */
	short?: string;
};

export const NAV: NavItem[] = [
	{ label: 'Overview', href: CONSOLE_ROUTES.overview, icon: LayoutDashboard },
	{ label: 'Posts', href: CONSOLE_ROUTES.posts, icon: FileText },
	{ label: 'Projects', href: CONSOLE_ROUTES.projects, icon: Folder },
	{ label: 'Résumés', href: CONSOLE_ROUTES.resumes, icon: User },
	{ label: 'Applications', href: CONSOLE_ROUTES.applications, icon: Briefcase, short: 'Apps' },
	{ label: 'Media', href: CONSOLE_ROUTES.media, icon: Image },
	{ label: 'Topics', href: CONSOLE_ROUTES.topics, icon: Tag }
];

/** Reached below the fold of the nav, after a rule. */
export const ACCOUNT: NavItem = {
	label: 'Account',
	href: CONSOLE_ROUTES.account,
	// icon/circle-user, per Screen / Overview 68:23. `User` is Résumés' glyph.
	icon: CircleUser
};

/**
 * The mobile bar: four destinations and a fifth slot for More.
 *
 * `Posts · Projects · Apps · Media · More`, identical on all fourteen console
 * frames that carry a bar — which is what makes it a decision rather than a
 * screenshot artefact. A wrapped tab row pushes the first result below the
 * fold, so five slots is what there is.
 */
export const TAB_BAR: NavItem[] = [
	NAV[1], // Posts
	NAV[2], // Projects
	NAV[4], // Applications, printed "Apps"
	NAV[5] // Media
];

/**
 * Behind More: the four destinations the bar has no room for.
 *
 * Overview first because it is the home screen — and until this sheet existed
 * there was no way to reach it at 390px at all, the bar having been read off
 * the frames without anyone noticing what it left out.
 *
 * The Prototype Map's mobile transform table is the ruling. The sheet itself
 * is the one mobile surface with no frame drawn; two are owed.
 */
export const MORE: NavItem[] = [
	NAV[0], // Overview
	NAV[3], // Résumés
	NAV[6], // Topics
	ACCOUNT
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

/**
 * What the mobile header shows on a given screen.
 *
 * There is no sidebar below 768px, so the header carries what it would have:
 * the section's name and its one amber action (Mobile / Posts list 73:2), or,
 * inside a section, a way back instead (Mobile / New post 97:2871, Mobile / Posts
 * archive 97:2744). A form that is being written hides the tab bar, as Mobile /
 * New post draws it — a half-written draft is not something to tab away from.
 *
 * Defined here, beside the nav, so a screen cannot declare a header the nav
 * disagrees with.
 */
export type MobileChrome = {
	title: string;
	back: string | null;
	action: { label: string; href: string } | null;
	tabs: boolean;
	/** The screen draws its own header, and the shell draws none. */
	bare?: boolean;
};

const SECTION_ACTIONS: Record<string, { label: string; href: string }> = {
	[CONSOLE_ROUTES.posts]: { label: 'New', href: `${CONSOLE_ROUTES.posts}/new` },
	[CONSOLE_ROUTES.applications]: { label: 'Add', href: `${CONSOLE_ROUTES.applications}/new` }
};

export function mobileChrome(path: string): MobileChrome {
	const here = path.replace(/\/+$/, '') || '/';
	const posts = CONSOLE_ROUTES.posts;

	if (here === `${posts}/new`) {
		return { title: 'New post', back: posts, action: null, tabs: false };
	}
	if (here === `${posts}/archive`) {
		return { title: 'Archive', back: posts, action: null, tabs: true };
	}
	if (here.startsWith(`${posts}/`)) {
		// Mobile / Post editor 73:236: the editor's own bar carries back, the
		// status pill and Publish, so the shell's would only be a second one.
		return { title: 'Edit post', back: posts, action: null, tabs: false, bare: true };
	}

	return {
		title: currentLabel(here),
		back: null,
		action: SECTION_ACTIONS[here] ?? null,
		tabs: true
	};
}
