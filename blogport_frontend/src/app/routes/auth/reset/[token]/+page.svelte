<script lang="ts">
	import { page } from '$app/state';
	import { ResetPasswordPage } from '$lib/pages/reset-password';

	/**
	 * `/auth/reset/[token]` — Console Blueprint §03, auth surfaces, and J3.
	 *
	 * The token stays in the URL and is never stored: a refused password must
	 * not cost a still-valid token.
	 */
	const token = $derived(page.params.token ?? '');
</script>

<svelte:head>
	<title>Set a new password</title>
</svelte:head>

<ResetPasswordPage {token} />
