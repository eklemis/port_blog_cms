<script lang="ts">
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { RegisterPage } from '$lib/pages/register';
	import { HOLD_ROUTE } from '$lib/shared/config/routes';

	/**
	 * `/auth/register` — Console Blueprint §03, auth surfaces, and J1 step 1.
	 *
	 * Registration returns a user and no token, so nobody is signed in
	 * afterwards. `invalidateAll` so the hold screen's load runs against the
	 * cookie the proxy just set, which is where it reads the address from.
	 */
	const registered = () => goto(resolve(HOLD_ROUTE), { invalidateAll: true });
</script>

<svelte:head>
	<title>Create your account</title>
</svelte:head>

<RegisterPage onregistered={registered} />
