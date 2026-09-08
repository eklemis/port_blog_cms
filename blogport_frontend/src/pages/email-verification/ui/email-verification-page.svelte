<script lang="ts">
	import { resolve } from '$app/paths';
	import { PRODUCT_NAME } from '$lib/shared/config/product';
	import { CONSOLE_ROUTE, SIGN_IN_ROUTE } from '$lib/shared/config/routes';
	import { ResendVerification } from '$lib/features/verify-email';

	/**
	 * Where the emailed verification link lands.
	 *
	 * The Console Blueprint specifies this screen in prose and never draws it,
	 * so it is built from the verification gate's card rather than invented: the
	 * same badge, headline, sentence and actions, because it is the same moment
	 * in the same journey and should not look like a different product.
	 *
	 * Copy marked below is not in the blueprint's table — see the PR.
	 */
	let {
		verified,
		/** Why it did not work. Absent when it did. */
		message,
		/** Whether a session exists, which is what makes a resend possible at all. */
		canResend = false
	}: { verified: boolean; message?: string; canResend?: boolean } = $props();

	let resendMessage = $state<string | undefined>();
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
				       {verified ? 'border-st-live text-st-live' : 'border-st-danger text-st-danger'}"
			>
				{verified ? 'Verified' : 'Link expired'}
			</p>

			<h1
				class="font-display text-[28px] leading-[34px] font-extrabold tracking-tight
				       text-arch-headline md:text-[26px] md:leading-tight"
			>
				<!-- Invented copy: the blueprint gives no sentence for the success. -->
				{verified ? 'Your email is verified' : message}
			</h1>

			<p class="text-[14.5px] leading-[23px] text-arch-muted md:text-[13.5px] md:leading-[22px]">
				{#if verified}
					<!-- Invented copy. -->
					Everything is unlocked: posts, projects, résumés and uploads.
				{:else if canResend}
					<!-- Invented copy. Verification links last JWT_VERIFICATION_EXPIRY, 24h. -->
					Verification links are good for 24 hours. Send yourself a new one.
				{:else}
					<!-- Invented copy. Not a bounce to login for its own sake: a resend
					     needs an address, and the session is the only place we have one. -->
					Verification links are good for 24 hours. Sign in and you can send yourself a new one.
				{/if}
			</p>

			<div class="flex flex-col">
				<!-- eslint-disable svelte/no-navigation-without-resolve --
					`resolve()` takes a route id that exists, and /studio is in the surface
					map without being built yet. Swap both in when it is. -->
				<div class="flex flex-col gap-[18px] md:flex-row md:items-start md:gap-[9px]">
					{#if verified}
						<a
							href={canResend ? CONSOLE_ROUTE : SIGN_IN_ROUTE}
							class="inline-flex min-h-11 items-center justify-center rounded-lg border
							       border-arch-line-control px-4 py-2 text-sm font-semibold
							       text-arch-headline hover:bg-arch-surface-2"
						>
							{canResend ? 'Go to your console' : 'Sign in'}
						</a>
					{:else if canResend}
						<ResendVerification onmessage={(m) => (resendMessage = m)} />
					{:else}
						<a
							href={SIGN_IN_ROUTE}
							class="inline-flex min-h-11 items-center justify-center rounded-lg border
							       border-arch-line-control px-4 py-2 text-sm font-semibold
							       text-arch-headline hover:bg-arch-surface-2"
						>
							Sign in
						</a>
					{/if}
				</div>

				<p role="status" class="text-[12px] text-arch-muted {resendMessage ? 'mt-3' : ''}">
					{resendMessage ?? ''}
				</p>
			</div>
		</div>
	</main>
</div>
