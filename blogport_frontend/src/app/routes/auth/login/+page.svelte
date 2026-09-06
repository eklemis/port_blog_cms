<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { SignInPage } from '$lib/pages/sign-in';

	/**
	 * `/auth/login` — Console Blueprint §03, auth surfaces.
	 *
	 * Two query parameters, both written by J2 when a session could not be
	 * refreshed: `next` is the destination to return to, and `reason=expired`
	 * is what makes the screen say why the person is looking at it.
	 *
	 * `next` is never trusted here — features/sign-in/model/gate decides what
	 * survives, and an unverified account does not reach it at all.
	 */
	const next = $derived(page.url.searchParams.get('next'));
	const sessionExpired = $derived(page.url.searchParams.get('reason') === 'expired');

	// `invalidateAll` so the load functions re-run against the cookies the proxy
	// just set — without it the console renders against the signed-out session.
	// The destination is decided at runtime by the gate and is already narrowed
	// to a same-site path there, so there is no route id for resolve() to take.
	// eslint-disable-next-line svelte/no-navigation-without-resolve
	const go = (destination: string) => goto(destination, { invalidateAll: true });
</script>

<svelte:head>
	<title>Sign in</title>
</svelte:head>

<SignInPage {next} {sessionExpired} onsignedin={go} />
