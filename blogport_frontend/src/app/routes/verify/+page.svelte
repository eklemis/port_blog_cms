<script lang="ts">
	import { goto } from '$app/navigation';
	import { VerifyPage } from '$lib/pages/verify';
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
	const signedOut = () => goto('/auth/login?reason=expired', { invalidateAll: true });
</script>

<svelte:head>
	<title>Verify your email</title>
</svelte:head>

<VerifyPage email={data.email} onsessionexpired={signedOut} />
