/**
 * English — the base catalogue.
 *
 * Keyed by what a string *is*, not by where it appears, so the same sentence is
 * not written twice and a screen that moves does not take its copy with it.
 *
 * §02: "Error copy is content, not code. The error-code table becomes a message
 * catalogue keyed by code — which is exactly why the blueprint insisted the raw
 * code never reaches the screen. That rule now pays for itself." The `error`
 * block below is that table.
 */
export const en = {
	language: {
		label: 'Language',
		en: 'English',
		id: 'Bahasa Indonesia'
	},
	error: {
		/** The fallback. Never a raw code, never "Something went wrong". */
		unexpected: 'Something went wrong on our side.',
		rateLimitedShortly: (lead: string) => `${lead}. Try again shortly.`,
		rateLimitedIn: (lead: string, amount: number, unit: 'second' | 'minute') =>
			`${lead}. Try again in ${amount} ${unit}${amount === 1 ? '' : 's'}.`,
		tooManyAttempts: 'Too many attempts'
	}
} as const;

export type Catalogue = typeof en;
