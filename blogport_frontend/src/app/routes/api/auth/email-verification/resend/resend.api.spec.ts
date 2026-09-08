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
 * The address comes from the request body when one is given, and from the
 * session otherwise.
 *
 * An earlier version of this route required a session. That was a hardening
 * decision of mine rather than a constraint the API imposes — the operation
 * carries no security requirement and takes `{ email }` — and it broke the
 * screen that needs it most: an expired link opened on a phone has no session,
 * which is exactly when a new one has to be asked for. The backend's own
 * defences are the right ones here: it answers 202 identically whether the
 * address is unknown, deleted, already verified or genuinely waiting, and it
 * rate-limits 5 per hour per caller.
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

test('takes the address from the body when one is given', async () => {
	// The expired-link screen asks for it, because a link opened from mail
	// usually has no session behind it.
	backendAccepts();

	await POST(event(null, { email: 'typed@example.com' }) as never);

	expect(backendPOST).toHaveBeenCalledWith('/api/auth/email-verification/resend', {
		body: { email: 'typed@example.com' }
	});
});

test('a supplied address wins over the session', async () => {
	// Otherwise someone signed in could not ask for a link to the address they
	// actually mistyped at registration.
	backendAccepts();

	await POST(event(SESSION, { email: 'typed@example.com' }) as never);

	expect(backendPOST).toHaveBeenCalledWith('/api/auth/email-verification/resend', {
		body: { email: 'typed@example.com' }
	});
});

test('trims the address before sending it', async () => {
	backendAccepts();

	await POST(event(null, { email: '  typed@example.com  ' }) as never);

	expect(backendPOST).toHaveBeenCalledWith('/api/auth/email-verification/resend', {
		body: { email: 'typed@example.com' }
	});
});

test('with neither an address nor a session there is nothing to send to', async () => {
	const response = await POST(event(null) as never);

	expect(response.status).toBe(400);
	expect(await response.json()).toMatchObject({ error: { code: 'MISSING_FIELD' } });
	expect(backendPOST).not.toHaveBeenCalled();
});

test('a blank address is not an address', async () => {
	const response = await POST(event(null, { email: '   ' }) as never);

	expect(response.status).toBe(400);
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
