import { beforeEach, expect, test, vi } from 'vitest';

const fetchImpl = vi.fn();

let unreachable: Error | null = null;

vi.mock('$lib/shared/config/backend', () => ({
	backendBaseUrl: 'http://backend'
}));

const { load } = await import('./+page.server');

/**
 * `/[username]/projects/[slug]` — one project, read by a stranger.
 *
 * `owner` on the payload is a bare UUID, so the header's name and avatar come
 * from `GET /api/public/users/{username}` — a supporting call §03's "Backed by"
 * column does not name. The fifth screen where that has been true.
 */

const PROJECT = {
	data: {
		id: 'p-1',
		slug: 'blogport-cms',
		title: 'Blogport CMS',
		description: '## What it is\n\nA portfolio CMS across four services.',
		tech_stack: ['Rust', 'SvelteKit'],
		topics: [{ id: 't-rust', title: 'Rust' }],
		repo_url: 'https://github.com/eklemis/port_blog_cms',
		live_demo_url: null,
		owner: '123e4567-e89b-12d3-a456-426614174000',
		screenshots: [],
		media: [
			{
				media_id: 'm1',
				alt_text: 'The console',
				role: 'cover',
				position: 0,
				caption: '',
				variants: { large: '/api/public/media/m1/large' }
			},
			{
				media_id: 'm2',
				alt_text: 'The editor',
				role: 'screenshot',
				position: 1,
				caption: '',
				variants: { large: '/api/public/media/m2/large' }
			}
		]
	}
};

const PROFILE = { data: { username: 'janedoe', full_name: 'Jane Doe', bio: null, avatar: null } };

function backend(
	answers: Partial<Record<'project' | 'profile', { status: number; body?: unknown }>> = {}
) {
	const project = answers.project ?? { status: 200, body: PROJECT };
	const profile = answers.profile ?? { status: 200, body: PROFILE };

	fetchImpl.mockImplementation(async (url: string) => {
		if (unreachable) throw unreachable;
		const answer = String(url).includes('/api/public/users/') ? profile : project;
		return {
			ok: answer.status < 400,
			status: answer.status,
			json: async () => answer.body ?? null
		};
	});
}

const event = (username = 'janedoe', slug = 'blogport-cms') => ({
	params: { username, slug },
	fetch: fetchImpl
});

beforeEach(() => {
	unreachable = null;
	fetchImpl.mockReset();
});

test('asks for the project and the author who owns the header', async () => {
	backend();

	await load(event() as never);

	const urls = fetchImpl.mock.calls.map((c) => String(c[0]));
	expect(urls).toContain('http://backend/api/public/projects/janedoe/blogport-cms');
	expect(urls).toContain('http://backend/api/public/users/janedoe');
});

test('renders the description on the server, because it is markdown', async () => {
	// §03 puts `description` in the markdown class, with `excerpt` the one
	// exception that stays plain.
	backend();

	const data = (await load(event() as never)) as { bodyHtml: string };

	expect(data.bodyHtml).toContain('<h2>What it is</h2>');
	expect(data.bodyHtml).not.toContain('## What it is');
});

test('puts the cover first, then the screenshots behind it', async () => {
	backend();

	const data = (await load(event() as never)) as { images: { src: string; alt: string }[] };

	expect(data.images.map((image) => image.alt)).toEqual(['The console', 'The editor']);
	expect(data.images[0].src).toBe('http://backend/api/public/media/m1/large');
});

test('falls back to the plain screenshot URLs when there is no media', async () => {
	// The two coexist: `media` is uploaded and carries sizes, `screenshots` is a
	// list of addresses on the row. A project with only the latter still has a
	// gallery.
	backend({
		project: {
			status: 200,
			body: { data: { ...PROJECT.data, media: [], screenshots: ['https://cdn.test/a.png'] } }
		}
	});

	const data = (await load(event() as never)) as { images: { src: string }[] };

	expect(data.images).toEqual([{ src: 'https://cdn.test/a.png', alt: '' }]);
});

test('escapes what came out of the path', async () => {
	backend();

	await load(event('jane doe', 'a/b') as never);

	expect(String(fetchImpl.mock.calls[0][0])).toContain('/api/public/projects/jane%20doe/a%2Fb');
});

test('a project that does not exist is a plain 404', async () => {
	backend({ project: { status: 404, body: { error: { code: 'PROJECT_NOT_FOUND' } } } });

	await expect(load(event() as never)).rejects.toMatchObject({ status: 404 });
});

test('a backend that broke is not a project that does not exist', async () => {
	backend({ project: { status: 500, body: { error: { code: 'INTERNAL_ERROR' } } } });

	await expect(load(event() as never)).rejects.toMatchObject({ status: 500 });
});
