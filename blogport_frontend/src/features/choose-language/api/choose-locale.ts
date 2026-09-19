import type { Locale } from '$lib/shared/i18n';

/**
 * Store a language choice and reload into it.
 *
 * The reload is the point rather than a shortcut: §02 says changing the UI
 * language is "instant, no data effect", and every page here is server-rendered
 * — so the server has to be asked again for the same page in the other
 * language. Swapping strings in place would leave the ones already sent.
 */
export async function chooseLocale(
	locale: Locale,
	fetchFn: typeof globalThis.fetch = (...args) => globalThis.fetch(...args)
): Promise<boolean> {
	try {
		const response = await fetchFn('/api/locale', {
			method: 'POST',
			headers: { 'content-type': 'application/json' },
			body: JSON.stringify({ locale })
		});

		return response.ok;
	} catch {
		return false;
	}
}
