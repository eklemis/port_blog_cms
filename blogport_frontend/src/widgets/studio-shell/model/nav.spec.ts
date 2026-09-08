import { expect, test } from 'vitest';
import { CONSOLE_ROUTES } from '$lib/shared/config/routes';
import { NAV, TAB_BAR, currentLabel, isCurrent } from './nav';

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

test('the tab bar is five items, the rest behind More', () => {
	// A wrapped tab row pushes content below the fold; five is what fits.
	expect(TAB_BAR).toHaveLength(5);
	expect(TAB_BAR.at(-1)?.label).toBe('More');
});

test('the tab bar carries the four the mobile frame puts on the bar', () => {
	// Résumés is the one the frame drops when Applications joins: a résumé is
	// edited at a desk, an application is checked on a phone.
	expect(TAB_BAR.map((item) => item.label)).toEqual([
		'Posts',
		'Projects',
		'Applications',
		'Media',
		'More'
	]);
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
