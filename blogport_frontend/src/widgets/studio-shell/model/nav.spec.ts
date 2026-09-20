import { existsSync, readdirSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { expect, test } from 'vitest';
import { CONSOLE_CREATE, CONSOLE_ROUTES } from '$lib/shared/config/routes';
import { CircleUser } from '@lucide/svelte';
import {
	ACCOUNT,
	BELOW,
	DESIGNED,
	MORE,
	NAV,
	TAB_BAR,
	currentLabel,
	isCurrent,
	mobileChrome
} from './nav';

/**
 * The console's navigation, defined once so the sidebar, the tablet rail and
 * the mobile tab bar cannot drift from each other.
 */

test('names the surfaces the route map sanctions, and only those', () => {
	// Seven, in the order the Prototype Map's sidebar row gives them. It shipped
	// with six because the Console Blueprint's route table did not carry the
	// Career Studio; the designer has since added it, and confirmed the frames
	// were right all along.
	expect(DESIGNED.map((item) => item.label)).toEqual([
		'Overview',
		'Posts',
		'Projects',
		'Résumés',
		'Applications',
		'Media',
		'Topics'
	]);
});

/**
 * A nav item is a claim that a screen is there.
 *
 * §02 makes the rule for the posts table: the Topics column "is not built; it
 * is never filled with em dashes, which would read as 'no topics' on every
 * row." A link to a route that does not exist is the same error in navigation
 * form, and its answer is a 404 from inside the person's own console.
 *
 * So the designed seven stay written down, and what is *offered* is what has a
 * screen. The filesystem is the oracle rather than this file's good intentions.
 */

const pageFor = (href: string) =>
	fileURLToPath(new URL(`../../../app/routes${href}/+page.svelte`, import.meta.url));

test('every destination the console offers is a screen that exists', () => {
	for (const item of [...NAV, ...BELOW]) {
		expect(existsSync(pageFor(item.href)), `${item.label} promises ${item.href}`).toBe(true);
	}
});

test('a console screen that exists is offered', () => {
	// The other direction: building a screen and forgetting the nav leaves it
	// reachable only by typing the address.
	const studio = fileURLToPath(new URL('../../../app/routes/studio', import.meta.url));
	const built = readdirSync(studio, { withFileTypes: true })
		.filter((entry) => entry.isDirectory() && !entry.name.startsWith('['))
		.map((entry) => `/studio/${entry.name}`)
		.filter((href) => existsSync(pageFor(href)));

	const offered = new Set([...NAV, ...BELOW].map((item) => item.href));

	for (const href of built) expect(offered.has(href), `${href} is built`).toBe(true);
});

test('every create screen the console links to exists', () => {
	// The gap that let a dead link ship: the checks above cover the top-level
	// sections and nothing below them, so `/studio/projects/new` was offered by
	// two buttons while no route answered it. Worse than a 404 — it matched the
	// editor's `[id]` with an id of "new", and told the person the project they
	// were about to create could not be found.
	for (const href of Object.values(CONSOLE_CREATE)) {
		expect(existsSync(pageFor(href)), `${href} is offered`).toBe(true);
	}
});

test('a create screen belongs to a section that exists', () => {
	const sections = new Set(NAV.map((item) => item.href));

	for (const href of Object.values(CONSOLE_CREATE)) {
		expect(sections.has(href.replace(/\/new$/, '')), `${href}'s section`).toBe(true);
	}
});

test('the bar and the sheet offer nothing the sidebar does not', () => {
	const offered = new Set([...NAV, ...BELOW].map((item) => item.href));

	for (const item of [...TAB_BAR, ...MORE]) {
		expect(offered.has(item.href), `${item.label} in a mobile surface`).toBe(true);
	}
});

test('More is still reachable when the bar has lost a tab', () => {
	// Filtering must never empty the sheet: Overview lives behind More, and at
	// 390px there is no other way to it.
	expect(MORE.length).toBeGreaterThan(0);
});

test('every item points somewhere the map names', () => {
	const known = Object.values(CONSOLE_ROUTES);
	for (const item of NAV) expect(known).toContain(item.href);
});

test('the bar keeps the frames’ order, minus what has no screen', () => {
	// Posts · Projects · Apps · Media, identical on all fourteen console frames
	// that carry a bar — which is what makes it a decision rather than a
	// screenshot artefact. The fifth slot is More, and More is not a
	// destination; it opens the sheet.
	//
	// Media is not built, so the bar is short rather than backfilled from the
	// sheet. Borrowing a tab would be this file redesigning the mobile bar
	// because a screen is late.
	expect(TAB_BAR.map((item) => item.short ?? item.label)).toEqual(['Posts', 'Projects', 'Apps']);

	const intended = ['Posts', 'Projects', 'Apps', 'Media'];
	const offered = TAB_BAR.map((item) => item.short ?? item.label);
	expect(offered).toEqual(intended.filter((label) => offered.includes(label)));
});

test('a slot that cannot fit the full name says the short one', () => {
	// "Apps" is what the frame prints. It is also the accessible name — an
	// aria-label of "Applications" would not contain the visible text, which
	// is exactly what 2.5.3 forbids.
	const apps = TAB_BAR.find((item) => item.href === CONSOLE_ROUTES.applications);

	expect(apps?.short).toBe('Apps');
	expect(apps?.label).toBe('Applications');
});

test('More holds what does not fit, Overview first', () => {
	// Overview first because it is the home screen — and until this sheet
	// exists, Mobile / Overview has no way in at all. Résumés and Account
	// belong here too and return the day they have screens.
	expect(MORE.map((item) => item.label)).toEqual(['Overview', 'Topics']);
});

test('between the bar and the sheet, nothing is unreachable at 390px', () => {
	// The invariant that matters: eight destinations, five slots. Anything
	// dropped from the bar has to turn up behind More.
	const reachable = [...TAB_BAR, ...MORE].map((item) => item.href).sort();
	const every = [...NAV, ...BELOW].map((item) => item.href).sort();

	expect(reachable).toEqual(every);
});

// ── which item is lit ──────────────────────────────────────────────────────

test('the current section is the one whose route the path is inside', () => {
	expect(isCurrent(CONSOLE_ROUTES.posts, '/studio/posts')).toBe(true);
	expect(isCurrent(CONSOLE_ROUTES.posts, '/studio/posts/new')).toBe(true);
	expect(isCurrent(CONSOLE_ROUTES.posts, '/studio/posts/abc-123')).toBe(true);
});

test('overview is lit only on overview, never on everything', () => {
	// `/studio` is a prefix of every console path, so a naive startsWith would
	// leave Overview lit on all eight screens.
	expect(isCurrent(CONSOLE_ROUTES.overview, '/studio')).toBe(true);
	expect(isCurrent(CONSOLE_ROUTES.overview, '/studio/posts')).toBe(false);
	expect(isCurrent(CONSOLE_ROUTES.overview, '/studio/media')).toBe(false);
});

test('a sibling whose name merely starts the same is not the current one', () => {
	expect(isCurrent('/studio/post', '/studio/posts')).toBe(false);
});

// ── what the screen is called ──────────────────────────────────────────────

test('the career studio is inside the console, not beside it', () => {
	// Its own section in the map, but the same shell and the same session
	// rules — so it lights the sidebar like everything else.
	expect(isCurrent(CONSOLE_ROUTES.applications, '/studio/applications')).toBe(true);
	expect(isCurrent(CONSOLE_ROUTES.applications, '/studio/applications/new')).toBe(true);
	expect(currentLabel('/studio/applications/abc/tailor')).toBe('Applications');
});

test('names the screen from the same list the nav is drawn from', () => {
	// The mobile bar names the screen; the tab bar lights it. Both read this,
	// so they cannot disagree — which they did when each page declared its own.
	expect(currentLabel('/studio')).toBe('Overview');
	expect(currentLabel('/studio/posts')).toBe('Posts');
	expect(currentLabel('/studio/posts/new')).toBe('Posts');
	expect(currentLabel('/studio/account')).toBe('Account');
});

test('somewhere unrecognised is still called something', () => {
	expect(currentLabel('/studio/nowhere')).toBe('Console');
});

test('a trailing slash does not change which item is lit', () => {
	expect(isCurrent(CONSOLE_ROUTES.posts, '/studio/posts/')).toBe(true);
	expect(isCurrent(CONSOLE_ROUTES.overview, '/studio/')).toBe(true);
});

// ── the mobile header ──────────────────────────────────────────────────────

test('Account has its own glyph, not the one Résumés already uses', () => {
	// Screen / Overview 68:23 draws icon/circle-user; `User` is Résumés'.
	expect(ACCOUNT.icon).toBe(CircleUser);
});

test('a section names itself, with its primary action beside it', () => {
	// Mobile / Posts list 73:2 and Mobile / Application tracker 73:120 put the
	// screen's one amber button in the header, shortened to fit.
	expect(mobileChrome('/studio/posts')).toEqual({
		title: 'Posts',
		back: null,
		action: { label: 'New', href: '/studio/posts/new' },
		tabs: true
	});
	expect(mobileChrome('/studio/applications').action).toEqual({
		label: 'Add',
		href: '/studio/applications/new'
	});
});

test('a screen inside a section gets a way back instead of the section name', () => {
	// Mobile / New post 97:2871 and Mobile / Posts archive 97:2744: "‹ New post",
	// "‹ Archive".
	expect(mobileChrome('/studio/posts/archive')).toEqual({
		title: 'Archive',
		back: '/studio/posts',
		action: null,
		tabs: true
	});
});

test('a form that is being written hides the tab bar', () => {
	// Mobile / New post draws no tab bar: a half-written draft is not something
	// to tab away from by accident.
	expect(mobileChrome('/studio/posts/new')).toMatchObject({
		title: 'New post',
		back: '/studio/posts',
		tabs: false
	});
});

test('the editor draws its own header, so the shell draws none', () => {
	// Mobile / Post editor 73:236: back, status pill and Publish in one bar
	// that belongs to the editor, and no tab bar.
	expect(mobileChrome('/studio/posts/post-1')).toMatchObject({ bare: true, tabs: false });
});

test('anywhere else is named from the nav, with nothing added', () => {
	expect(mobileChrome('/studio')).toEqual({
		title: 'Overview',
		back: null,
		action: null,
		tabs: true
	});
	expect(mobileChrome('/studio/topics')).toEqual({
		title: 'Topics',
		back: null,
		action: null,
		tabs: true
	});
});
