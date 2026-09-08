/**
 * The routes more than one slice needs to name.
 *
 * They live here rather than in a feature because slices in the same layer may
 * not import each other: sign-in decides where a session lands, and the hold
 * screen decides who it lets go — both need the same two destinations, and
 * neither may reach into the other for them.
 *
 * Console Blueprint §03, surface map.
 */

/** Overview. The console's front door, and only for a verified account. */
export const CONSOLE_ROUTE = '/studio';

/** The hold screen: what verification unlocks, the address it went to, a resend. */
export const HOLD_ROUTE = '/verify';

export const SIGN_IN_ROUTE = '/auth/login';
