/**
 * Turning an HTTP failure into a sentence a person can act on.
 *
 * Lives in `shared` because more than one feature needs it and slices in the
 * same layer may not import each other — sign-in and verify-email both meet a
 * rate limit and both meet a server that fell over.
 *
 * The per-code copy stays with the feature that owns the endpoint; only what is
 * genuinely common is here. Console Blueprint §07.
 */

/** The fallback. Never a raw error code, never "Something went wrong". */
export const UNEXPECTED = 'Something went wrong on our side.';

/**
 * "Too many attempts. Try again in 42 minutes." — whole units, real countdown.
 *
 * Rate limits are a designed experience: read `Retry-After`, disable submit and
 * count down visibly. A dead button with no explanation reads as a broken
 * product at exactly the moment someone is already annoyed.
 */
export function rateLimited(seconds: number | null): string {
	if (seconds === null || seconds <= 0) return 'Too many attempts. Try again shortly.';

	const [amount, unit] = seconds < 60 ? [seconds, 'second'] : [Math.ceil(seconds / 60), 'minute'];

	return `Too many attempts. Try again in ${amount} ${unit}${amount === 1 ? '' : 's'}.`;
}

/** `Retry-After` in seconds, or null when the header is absent or nonsense. */
export function retryAfterSeconds(response: Response): number | null {
	const header = response.headers.get('retry-after');
	if (!header) return null;

	const seconds = Number(header);
	return Number.isFinite(seconds) && seconds > 0 ? seconds : null;
}
