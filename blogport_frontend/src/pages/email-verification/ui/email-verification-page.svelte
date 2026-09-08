<script lang="ts">
	import { resolve } from '$app/paths';
	import { PRODUCT_NAME } from '$lib/shared/config/product';
	import { CONSOLE_ROUTE, SIGN_IN_ROUTE } from '$lib/shared/config/routes';
	import { RequestNewLink } from '$lib/features/verify-email';

	/**
	 * Where the emailed verification link lands.
	 *
	 * The Console Blueprint specifies this screen in prose and never draws it,
	 * so it is built from the verification gate's card rather than invented: the
	 * same badge, headline, sentence and actions, because it is the same moment
	 * in the same journey and should not look like a different product. The copy
	 * is the designer's.
	 *
	 * One state the design asks for is missing. "Already verified" should read
	 * as a success rather than a scolding, and the API cannot distinguish it:
	 * `activate_user` is called unconditionally and its result maps only to
	 * not-found or database-error, so a first verification and a second click on
	 * the same link are byte-identical on the wire. Both land on "You're
	 * verified", which at least keeps the do-not-scold rule. See the PR.
	 */
	let {
		verified,
		/** Why it did not work. Absent when it did. */
		message,
		/** Whether a session exists — it decides where "next" points, nothing more. */
		hasSession = false,
		/** The link is past using, as opposed to a fault on our side. */
		dead = false
	}: { verified: boolean; message?: string; hasSession?: boolean; dead?: boolean } = $props();

	/** Set once a new link has been asked for, which replaces the form. */
	let sent = $state<string | undefined>();

	const settled = $derived(verified || Boolean(sent));
</script>

<div class="flex min-h-screen flex-col bg-arch-bg">
	<a
		href="#main"
		class="sr-only focus:not-sr-only focus:absolute focus:top-2 focus:left-2 focus:z-10
		       focus:rounded-lg focus:border focus:border-arch-line-control focus:bg-arch-surface
		       focus:px-3 focus:py-2 focus:text-[13px] focus:text-arch-headline"
	>
		Skip to content
	</a>

	<header
		class="flex h-[60px] shrink-0 items-center justify-between px-5 py-4
		       md:h-auto md:px-[30px] md:py-[22px]"
	>
		<a
			href={resolve('/')}
			class="font-display text-[18px] font-extrabold tracking-tight text-arch-headline
			       md:text-[19px]"
		>
			{PRODUCT_NAME}
		</a>
	</header>

	<main
		id="main"
		class="flex flex-1 flex-col px-5 pb-12 md:items-center md:justify-center md:px-6 md:py-10"
	>
		<div
			class="flex w-full flex-col gap-[18px] md:max-w-[520px] md:rounded-[14px] md:border
			       md:border-arch-line md:bg-arch-surface md:p-10"
		>
			<!-- Sentence case in the DOM, uppercased in CSS, so a screen reader
			     reads a phrase rather than spelling out an acronym. -->
			<p
				class="inline-flex self-start rounded-full border px-2.5 py-1 font-mono text-[9px]
				       tracking-[0.9px] uppercase
				       {settled ? 'border-st-live text-st-live' : 'border-st-danger text-st-danger'}"
			>
				{verified ? 'Verified' : sent ? 'Sent' : 'Link expired'}
			</p>

			<h1
				class="font-display text-[28px] leading-[34px] font-extrabold tracking-tight
				       text-arch-headline md:text-[26px] md:leading-tight"
			>
				{#if verified}
					You're verified.
				{:else if sent}
					Sent, if it was needed.
				{:else}
					{message}
				{/if}
			</h1>

			<p class="text-[14.5px] leading-[23px] text-arch-muted md:text-[13.5px] md:leading-[22px]">
				{#if verified}
					Your account is ready. Posts, projects, résumés and uploads are all open to you now.
				{:else if sent}
					{sent}
				{:else if dead}
					Verification links are good for 24 hours. Enter your address and we'll send a new one.
				{/if}
			</p>

			<!-- eslint-disable svelte/no-navigation-without-resolve --
				`resolve()` takes a route id that exists, and /studio is in the surface
				map without being built yet. Swap it in when it is. -->
			{#if settled}
				<div class="flex flex-col md:flex-row md:items-start">
					<a
						href={hasSession ? CONSOLE_ROUTE : SIGN_IN_ROUTE}
						class="inline-flex min-h-11 items-center justify-center rounded-lg border
						       border-arch-line-control px-4 py-2 text-sm font-semibold
						       text-arch-headline hover:bg-arch-surface-2"
					>
						{hasSession ? 'Go to your studio' : 'Sign in'}
					</a>
				</div>
			{:else if dead}
				<RequestNewLink onsent={(m) => (sent = m)} />
			{/if}
		</div>
	</main>
</div>
