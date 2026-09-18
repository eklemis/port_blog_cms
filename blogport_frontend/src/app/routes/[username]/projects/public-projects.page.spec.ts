import { beforeEach, expect, test, vi } from 'vitest';

const fetchImpl = vi.fn();

let unreachable: Error | null = null;

vi.mock('$lib/shared/config/backend', () => ({
	backendBaseUrl: 'http://backend'
}));

const { load } = await import('./+page.server');

/** `/[username]/projects` — an author's projects, read by a stranger. */

const PROJECTS = {
	data: {
		total: 8,
		page: 1,
		per_page: 10,
		items: [
			{
				id: 'p-1',
				slug: 'blogport-cms',
				title: 'Blogport CMS',
				description: 'A portfolio CMS across four deployed services.',
				tech_stack: ['Rust', 'SvelteKit'],
				topics: [{ id: 't-rust', title: 'Rust' }],
				repo_url: 'https://github.com/eklemis/port_blog_cms',
				live_demo_url: null,
				cover: {
					media_id: 'c1',
					alt_text: 'A screenshot',
					variants: { large: '/api/public/media/c1/large' }
				}
			}
		]
	}
};

const PROFILE = {
	data: { username: 'janedoe', full_name: 'Jane Doe', bio: null, avatar: null }
};

function backend(
	answers: Partial<Record<'projects' | 'profile', { status: number; body?: unknown }>> = {}
) {
	const projects = answers.projects ?? { status: 200, body: PROJECTS };
	const profile = answers.profile ?? { status: 200, body: PROFILE };

	fetchImpl.mockImplementation(async (url: string) => {
		if (unreachable) throw unreachable;
		const answer = String(url).includes('/api/public/users/') ? profile : projects;
		return {
			ok: answer.status < 400,
			status: answer.status,
			json: async () => answer.body ?? null
		};
	});
}

const event = (username = 'janedoe', search = '') => ({
	params: { username },
	url: new URL(`http://app.test/${username}/projects${search}`),
	fetch: fetchImpl
});

beforeEach(() => {
	unreachable = null;
	fetchImpl.mockReset();
});

test('asks the public endpoints, with no session of any kind', async () => {
	backend();

	await load(event() as never);

	const urls = fetchImpl.mock.calls.map((c) => String(c[0]));
	expect(urls.some((u) => u.startsWith('http://backend/api/public/projects/janedoe?'))).toBe(true);
	expect(urls).toContain('http://backend/api/public/users/janedoe');
});

test('hands the page a card it can draw without asking again', async () => {
	backend();

	const data = (await load(event() as never)) as {
		projects: { description: string; techStack: string[]; cover: { src: string } | null }[];
	};

	expect(data.projects[0].description).toBe('A portfolio CMS across four deployed services.');
	expect(data.projects[0].techStack).toEqual(['Rust', 'SvelteKit']);
	expect(data.projects[0].cover?.src).toBe('http://backend/api/public/media/c1/large');
});

test('names the active filter from the cards it came back with', async () => {
	// Every card now carries its topics, which is what makes the chip nameable.
	backend();

	const data = (await load(event('janedoe', '?topic_id=t-rust') as never)) as {
		filter: { id: string; title: string } | null;
	};

	expect(data.filter).toEqual({ id: 't-rust', title: 'Rust' });
});

test('passes the topic to the server rather than filtering the page', async () => {
	backend();

	await load(event('janedoe', '?topic_id=t-rust') as never);

	const listing = fetchImpl.mock.calls
		.map((c) => String(c[0]))
		.find((u) => u.includes('/api/public/projects/janedoe?'));
	expect(listing).toContain('topic_id=t-rust');
});

test('an author nobody has is a plain 404 that names no username', async () => {
	backend({ profile: { status: 404, body: { error: { code: 'USER_NOT_FOUND' } } } });

	await expect(load(event() as never)).rejects.toMatchObject({ status: 404 });
});

test('a backend that broke is not an author who does not exist', async () => {
	backend({ projects: { status: 500, body: { error: { code: 'INTERNAL_ERROR' } } } });

	await expect(load(event() as never)).rejects.toMatchObject({ status: 500 });
});
