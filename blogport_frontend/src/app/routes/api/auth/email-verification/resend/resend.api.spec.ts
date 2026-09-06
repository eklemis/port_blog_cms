import { beforeEach, expect, test, vi } from 'vitest';

const backendPOST = vi.fn();

/**
 * Set to make the client throw instead of answering — an unreachable backend.
 * It is a plain variable rather than `backendPOST.mockRejectedValue`, because a
 * `vi.fn` whose implementation rejects is also tracked by Vitest for its settled
 * result, and that second promise is reported as an unhandled rejection even
 * when the route under test catches the first one.
 */
let unreachable: Error | null = null;

vi.mock('$lib/shared/api/client', () => ({
	POST: async (...args: unknown[]) => {
		if (unreachable) throw unreachable;
		return backendPOST(...args);
	}
}));

const { POST } = await import('./+server');

/**
 * `POST /api/auth/email-verification/resend` — the proxy for the one action the
 * hold screen offers.
 *
 * The address is taken from the session and never from the request body. The
 * backend endpoint is public and deliberately non-committal so that it cannot
 * be used to discover which addresses are registered; putting a body-supplied
 * address through this proxy would make our own origin a way to trigger mail to
 * any address someone likes, which is a different problem the backend's
 * anti-oracle answer does not solve.
 */

const SESSION = {
	user_id: '123e4567-e89b-12d3-a456-426614174000',
	email: 'jane@example.com',
	username: 'janedoe',
	full_name: 'Jane Doe',
	bio: null,
	locale: 'en'
};

const ACCEPTED = 'If that address needs verifying, a new link is on its way.';

function event(user: typeof SESSION | null = SESSION, body: unknown = undefined) {
	return {
		locals: { user },
		request: new Request('http://localhost/api/auth/email-verification/resend', {
			method: 'POST',
			body: body === undefined ? null : JSON.stringify(body)
		})
	};
}

function backendAccepts() {
	backendPOST.mockResolvedValue({
		data: { data: { message: ACCEPTED }, success: true },
		error: undefined,
		response: new Response(null, { status: 202 })
	});
}

function backendFails(status: number, code: string, headers = {}) {
	backendPOST.mockResolvedValue({
		data: undefined,
		error: { error: { code, message: 'whatever the server said' }, success: false },
		response: new Response(null, { status, headers })
	});
}

beforeEach(() => {
	backendPOST.mockReset();
	unreachable = null;
});

test('an unreachable backend answers with a code, not an unhandled throw', async () => {
	unreachable = new TypeError('fetch failed');

	const response = await POST(event() as never);

	expect(response.status).toBe(502);
	expect(await response.json()).toMatchObject({ error: { code: 'INTERNAL_ERROR' } });
});

test('asks the backend to re-send to the signed-in address', async () => {
	backendAccepts();

	const response = await POST(event() as never);

	expect(backendPOST).toHaveBeenCalledWith('/api/auth/email-verification/resend', {
		body: { email: 'jane@example.com' }
	});
	expect(response.status).toBe(202);
	expect(await response.json()).toEqual({ message: ACCEPTED });
});

test('ignores an address supplied in the body', async () => {
	// Otherwise this route is a way to make our server mail anyone.
	backendAccepts();

	await POST(event(SESSION, { email: 'someone-else@example.com' }) as never);

	expect(backendPOST).toHaveBeenCalledWith('/api/auth/email-verification/resend', {
		body: { email: 'jane@example.com' }
	});
});

test('refuses without a session rather than guessing an address', async () => {
	const response = await POST(event(null) as never);

	expect(response.status).toBe(401);
	expect(backendPOST).not.toHaveBeenCalled();
});

test('answers 202 the way the endpoint does — accepted, and nothing more', async () => {
	// 202 rather than 200 because that is what it means: the request was taken,
	// and nothing about what followed is being reported.
	backendAccepts();

	const response = await POST(event() as never);

	expect(response.status).toBe(202);
});

test('passes a rate limit through with its Retry-After intact', async () => {
	backendFails(429, 'RATE_LIMITED', { 'retry-after': '3600' });

	const response = await POST(event() as never);

	expect(response.status).toBe(429);
	expect(response.headers.get('retry-after')).toBe('3600');
	expect(await response.json()).toMatchObject({ error: { code: 'RATE_LIMITED' } });
});
