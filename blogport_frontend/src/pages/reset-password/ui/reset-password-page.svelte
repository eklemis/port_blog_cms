<script lang="ts">
	import { resolve } from '$app/paths';
	import { PRODUCT_NAME } from '$lib/shared/config/product';
	import { SIGN_IN_ROUTE } from '$lib/shared/config/routes';
	import { Button } from '$lib/shared/ui';
	import { RESET_LINK_DEAD, SetPasswordForm } from '$lib/features/password-reset';

	/**
	 * `/auth/reset/[token]` — set a new password from the emailed link.
	 *
	 * Two things the design asks for are not here, because nothing exposes the
	 * address behind a reset token: there is no GET on the token, and the 200
	 * carries only a message. So "For jane@example.com." is dropped, and J3's
	 * "route to login, prefilled" cannot prefill. See the PR.
	 *
	 * Design: Screen / Reset password 67:174 · Mobile 89:2249.
	 */
	let { token }: { token: string } = $props();

	let done = $state(false);
	let expired = $state(false);
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
			class="flex w-full flex-col gap-[18px] md:max-w-[408px] md:gap-5 md:rounded-[14px]
			       md:border md:border-arch-line md:bg-arch-surface md:px-[34px] md:py-[32px]"
		>
			{#if done}
				<p
					class="inline-flex self-start rounded-full border border-st-live px-2.5 py-1
					       font-mono text-[9px] tracking-[0.9px] text-st-live uppercase"
				>
					Changed
				</p>

				<h1
					class="font-display text-[29px] leading-[35px] font-extrabold tracking-tight
					       text-arch-headline md:text-[26px] md:leading-tight"
				>
					Password changed.
				</h1>

				<!--
					Invented copy, but the revocation is the API's own behaviour and
					worth saying: someone resetting a password often does it because
					they think somebody else had it.
				-->
				<p
					class="text-[14.5px] leading-[23px] text-arch-muted md:text-[13.5px]
					       md:leading-[22px]"
				>
					Every other session was signed out. Sign in with your new password.
				</p>

				<div class="flex flex-col md:flex-row md:items-start">
					<Button kind="secondary" label="Sign in" href={SIGN_IN_ROUTE} />
				</div>
			{:else if expired}
				<p
					class="inline-flex self-start rounded-full border border-st-danger px-2.5 py-1
					       font-mono text-[9px] tracking-[0.9px] text-st-danger uppercase"
				>
					Link expired
				</p>

				<h1
					class="font-display text-[29px] leading-[35px] font-extrabold tracking-tight
					       text-arch-headline md:text-[26px] md:leading-tight"
				>
					{RESET_LINK_DEAD}
				</h1>

				<!-- Invented copy. Reset tokens last JWT_PASSWORD_RESET_EXPIRY, one hour. -->
				<p
					class="text-[14.5px] leading-[23px] text-arch-muted md:text-[13.5px]
					       md:leading-[22px]"
				>
					Reset links are good for one hour. Ask for a new one and it will arrive in a moment.
				</p>

				<!-- J3: offer a fresh one from this same screen, never a bounce to login. -->
				<div class="flex flex-col md:flex-row md:items-start">
					<Button kind="secondary" label="Send a new link" href="/auth/forgot" />
				</div>
			{:else}
				<h1
					class="font-display text-[29px] leading-[35px] font-extrabold tracking-tight
					       text-arch-headline md:text-[26px] md:leading-tight"
				>
					Set a new password
				</h1>

				<SetPasswordForm {token} onset={() => (done = true)} onexpired={() => (expired = true)} />

				<p
					class="rounded-lg border border-st-inflight px-3.5 py-2.5 text-[11px]
					       leading-[17px] text-st-inflight"
				>
					If this link has expired you can request a new one — the token stays in the URL until
					then.
				</p>
			{/if}
		</div>
	</main>
</div>
