import { beforeEach, expect, test, vi } from 'vitest';

const backendGET = vi.fn();

/** Set to make the client throw — an unreachable backend. See resend.api.spec.ts. */
let unreachable: Error | null = null;

vi.mock('$lib/shared/api/client', () => ({
	GET: async (...args: unknown[]) => {
		if (unreachable) throw unreachable;
		return backendGET(...args);
	}
}));

const { load } = await import('./+page.server');

/**
 * The emailed link lands here. It is a plain GET with the token in the path, so
 * this route calls the API itself and renders the outcome — there is nothing to
 * store and no session to establish.
 *
 * Two outcomes reach the screen, not three. The backend answers 200 whether the
 * address was just verified or had been verified already, which is exactly what
 * the blueprint asks for: "already verified" is a success, not something to
 * scold someone for.
 */

const SESSION = {
	user_id: '123e4567-e89b-12d3-a456-426614174000',
	email: 'jane@example.com',
	username: 'janedoe',
	full_name: 'Jane Doe',
	bio: null,
	locale: 'en',
	is_verified: false
};

function event(token = 'a-token', user: typeof SESSION | null = null) {
	return { params: { token }, locals: { user } };
}

function backendVerifies() {
	backendGET.mockResolvedValue({
		data: { data: { message: 'Email verified successfully' }, success: true },
		error: undefined,
		response: new Response(null, { status: 200 })
	});
}

function backendRefuses(status: number, code: string) {
	backendGET.mockResolvedValue({
		data: undefined,
		error: { error: { code, message: 'whatever the server said' }, success: false },
		response: new Response(null, { status })
	});
}

beforeEach(() => {
	backendGET.mockReset();
	unreachable = null;
});

test('sends the token from the path to the API', async () => {
	backendVerifies();

	await load(event('the-emailed-token') as never);

	expect(backendGET).toHaveBeenCalledWith('/api/auth/email-verification/{token}', {
		params: { path: { token: 'the-emailed-token' } }
	});
});

test('a good link verifies', async () => {
	backendVerifies();

	expect(await load(event() as never)).toMatchObject({ verified: true });
});

test('a link used twice is still a success, and says nothing scolding', async () => {
	// The backend answers 200 for an address that was already verified, so this
	// is the same branch. Nothing here may treat coming back as a mistake.
	backendVerifies();

	const data = await load(event() as never);

	expect(data).toMatchObject({ verified: true });
	expect(JSON.stringify(data)).not.toMatch(/already|again|mistake/i);
});

test.each([
	['TOKEN_EXPIRED', 400],
	['TOKEN_INVALID', 400],
	['USER_NOT_FOUND', 404]
])('%s leaves the link unusable, with the blueprint’s sentence', async (code, status) => {
	backendRefuses(status, code);

	const data = (await load(event() as never)) as { verified: boolean; message: string };

	expect(data.verified).toBe(false);
	expect(data.message).toBe('This link has expired.');
});

test('a server fault is ours, and is not blamed on the link', async () => {
	backendRefuses(500, 'INTERNAL_ERROR');

	const data = (await load(event() as never)) as { verified: boolean; message: string };

	expect(data.verified).toBe(false);
	expect(data.message).toBe('Something went wrong on our side.');
});

test('an unreachable backend does not throw the emailed link at a 500 page', async () => {
	unreachable = new TypeError('fetch failed');

	const data = (await load(event() as never)) as { verified: boolean; message: string };

	expect(data.verified).toBe(false);
	expect(data.message).toBe('Something went wrong on our side.');
});

test('tells the screen where "next" points', async () => {
	// Not whether a resend is possible — it always is now, because the screen
	// asks for the address. Only whether there is a session to return to.
	backendRefuses(400, 'TOKEN_EXPIRED');

	expect(await load(event('t', null) as never)).toMatchObject({ hasSession: false });
	expect(await load(event('t', SESSION) as never)).toMatchObject({ hasSession: true });
});

test('separates a stale link from a fault on our side', async () => {
	// The screen offers a new link for one and not the other: asking for another
	// link when the server is broken just fails the same way.
	backendRefuses(400, 'TOKEN_EXPIRED');
	expect(await load(event() as never)).toMatchObject({ dead: true });

	backendRefuses(500, 'INTERNAL_ERROR');
	expect(await load(event() as never)).toMatchObject({ dead: false });
});
