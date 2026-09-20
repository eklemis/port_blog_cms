import { expect, test } from 'vitest';
import { linksOf, type ProjectCard } from './project';

/**
 * A project as the console lists it — Screen / Projects list 70:2.
 *
 * The one thing the row has to decide is its Links cell. The frame draws
 * "repo · demo", "repo" alone, "demo" alone, and a project may have neither.
 */

const project = (over: Partial<ProjectCard> = {}): ProjectCard => ({
	id: 'p-1',
	title: 'Blogport CMS',
	slug: 'blogport-cms',
	description: 'A portfolio CMS across four services.',
	tech_stack: ['Rust', 'SvelteKit'],
	topics: [],
	repo_url: 'https://github.com/eklemis/port_blog_cms',
	live_demo_url: 'https://blogport.example.test',
	updated_at: '2026-09-18T09:00:00Z',
	...over
});

test('names both links when a project has both', () => {
	expect(linksOf(project()).map((link) => link.label)).toEqual(['repo', 'demo']);
});

test('repo first, always — it is the one a reader of code wants', () => {
	// The frame draws "repo · demo" in that order on every row that has both.
	const [first] = linksOf(project());

	expect(first.label).toBe('repo');
	expect(first.href).toBe('https://github.com/eklemis/port_blog_cms');
});

test('a project with only one link shows only that one', () => {
	expect(linksOf(project({ live_demo_url: null })).map((l) => l.label)).toEqual(['repo']);
	expect(linksOf(project({ repo_url: null })).map((l) => l.label)).toEqual(['demo']);
});

test('a project with neither shows no links rather than an empty separator', () => {
	// "·" on its own is punctuation with nothing to separate.
	expect(linksOf(project({ repo_url: null, live_demo_url: null }))).toEqual([]);
});

test('a blank URL is not a link', () => {
	// The editor's Live demo field is empty in the frame, and an empty string
	// round-trips through the API as a value rather than as absence.
	expect(linksOf(project({ repo_url: '   ', live_demo_url: '' }))).toEqual([]);
});
