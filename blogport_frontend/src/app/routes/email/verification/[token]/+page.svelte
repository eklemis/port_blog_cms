<script lang="ts">
	import { EmailVerificationPage } from '$lib/pages/email-verification';
	import type { PageProps } from './$types';

	/**
	 * `/email/verification/[token]` — Console Blueprint §03, auth surfaces, and
	 * journey J1 step 3. The work happens in +page.server.ts so the emailed link
	 * resolves without JavaScript.
	 */
	let { data }: PageProps = $props();
</script>

<svelte:head>
	<title>{data.verified ? 'Email verified' : 'Verification link'}</title>
</svelte:head>

<EmailVerificationPage
	verified={data.verified}
	message={'message' in data ? data.message : undefined}
	dead={data.dead}
	hasSession={data.hasSession}
/>
