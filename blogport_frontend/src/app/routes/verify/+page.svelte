<script lang="ts">
	import { goto } from '$app/navigation';
	import { VerifyPage } from '$lib/pages/verify';
	import { SIGN_IN_ROUTE } from '$lib/shared/config/routes';
	import type { PageProps } from './$types';

	/**
	 * `/verify` — the hold screen. Console Blueprint §02 and journey J1.
	 *
	 * Reached from sign-in when the login response says the account is not
	 * verified, and from registration once that screen exists.
	 */
	let { data }: PageProps = $props();

	// J2: an unrecoverable 401 saves the destination and says why. Coming back
	// here afterwards is right — the gate decides where they actually land.
	// eslint-disable-next-line svelte/no-navigation-without-resolve
	const sessionExpired = () => goto('/auth/login?reason=expired', { invalidateAll: true });

	// `invalidateAll` so the load runs again against the cleared cookies; without
	// it the hold screen would render once more against a session that is gone.
	// eslint-disable-next-line svelte/no-navigation-without-resolve
	const onsignedout = () => goto(SIGN_IN_ROUTE, { invalidateAll: true });
</script>

<svelte:head>
	<title>Verify your email</title>
</svelte:head>

<VerifyPage email={data.email} onsessionexpired={sessionExpired} {onsignedout} />
