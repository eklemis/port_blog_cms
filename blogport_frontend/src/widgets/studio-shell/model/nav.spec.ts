import { expect, test } from 'vitest';
import { CONSOLE_ROUTES } from '$lib/shared/config/routes';
import { CircleUser } from '@lucide/svelte';
import { ACCOUNT, MORE, NAV, TAB_BAR, currentLabel, isCurrent, mobileChrome } from './nav';

/**
 * The console's navigation, defined once so the sidebar, the tablet rail and
 * the mobile tab bar cannot drift from each other.
 */

test('lists the surfaces the route map sanctions, and only those', () => {
	// Seven, in the order the Prototype Map's sidebar row gives them. It shipped
	// with six because the Console Blueprint's route table did not carry the
	// Career Studio; the designer has since added it, and confirmed the frames
	// were right all along.
	expect(NAV.map((item) => item.label)).toEqual([
		'Overview',
		'Posts',
		'Projects',
		'Résumés',
		'Applications',
		'Media',
		'Topics'
	]);
});

test('every item points somewhere the map names', () => {
	const known = Object.values(CONSOLE_ROUTES);
	for (const item of NAV) expect(known).toContain(item.href);
});

test('the bar is the four destinations the mobile frames draw', () => {
	// Identical on all fourteen console frames that carry a bar, which is what
	// makes it a decision rather than a screenshot artefact. The fifth slot is
	// More, and More is not a destination — it opens the sheet.
	expect(TAB_BAR.map((item) => item.short ?? item.label)).toEqual([
		'Posts',
		'Projects',
		'Apps',
		'Media'
	]);
});

test('a slot that cannot fit the full name says the short one', () => {
	// "Apps" is what the frame prints. It is also the accessible name — an
	// aria-label of "Applications" would not contain the visible text, which
	// is exactly what 2.5.3 forbids.
	const apps = TAB_BAR.find((item) => item.href === CONSOLE_ROUTES.applications);

	expect(apps?.short).toBe('Apps');
	expect(apps?.label).toBe('Applications');
});

test('More holds the four that do not fit, Overview first', () => {
	// Overview first because it is the home screen — and until this sheet
	// exists, Mobile / Overview has no way in at all.
	expect(MORE.map((item) => item.label)).toEqual(['Overview', 'Résumés', 'Topics', 'Account']);
});

test('between the bar and the sheet, nothing is unreachable at 390px', () => {
	// The invariant that matters: eight destinations, five slots. Anything
	// dropped from the bar has to turn up behind More.
	const reachable = [...TAB_BAR, ...MORE].map((item) => item.href).sort();
	const every = [...NAV, ACCOUNT].map((item) => item.href).sort();

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
	expect(mobileChrome('/studio/posts/post-1')).toMatchObject({
		back: '/studio/posts',
		tabs: false
	});
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
