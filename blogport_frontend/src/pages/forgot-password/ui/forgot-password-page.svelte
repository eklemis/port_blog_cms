<script lang="ts">
	import { resolve } from '$app/paths';
	import { PRODUCT_NAME } from '$lib/shared/config/product';
	import { SIGN_IN_ROUTE } from '$lib/shared/config/routes';
	import { Button } from '$lib/shared/ui';
	import { RequestResetForm, resetRequested } from '$lib/features/password-reset';

	/**
	 * `/auth/forgot` — ask for a reset link, then the confirmation.
	 *
	 * The confirmation is the whole point of J3: it reads the same whether or
	 * not that address has an account, because branching would turn the form
	 * into a way of discovering who is registered.
	 *
	 * Only the sent state is drawn (Screen / Forgot password 67:136, Mobile
	 * 89:2209); the form before it is built from the same card grammar.
	 */
	let sentTo = $state<string | undefined>();
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
			class="flex w-full flex-col gap-[18px] md:max-w-[430px] md:gap-5 md:rounded-[14px]
			       md:border md:border-arch-line md:bg-arch-surface md:px-[34px] md:py-[32px]"
		>
			{#if sentTo}
				<p
					class="inline-flex self-start rounded-full border border-st-live px-2.5 py-1
					       font-mono text-[9px] tracking-[0.9px] text-st-live uppercase"
				>
					Sent
				</p>

				<h1
					class="font-display text-[29px] leading-[35px] font-extrabold tracking-tight
					       text-arch-headline md:text-[26px] md:leading-tight"
				>
					Check your email
				</h1>

				<p
					class="text-[14.5px] leading-[23px] break-words text-arch-muted
					       md:text-[13.5px] md:leading-[22px]"
				>
					{resetRequested(sentTo)}
				</p>

				<div class="flex flex-col gap-[18px] md:flex-row md:items-start md:gap-2">
					<Button kind="secondary" label="Back to sign in" href={SIGN_IN_ROUTE} />
					<Button kind="ghost" label="Send again" onclick={() => (sentTo = undefined)} />
				</div>
			{:else}
				<h1
					class="font-display text-[29px] leading-[35px] font-extrabold tracking-tight
					       text-arch-headline md:text-[26px] md:leading-tight"
				>
					Reset your password
				</h1>

				<!-- Invented copy: only the sent state is drawn. See the PR. -->
				<p
					class="text-[14.5px] leading-[23px] text-arch-muted md:text-[13.5px]
					       md:leading-[22px]"
				>
					Enter the address you signed up with and we'll send a link to set a new one.
				</p>

				<RequestResetForm onsent={(email) => (sentTo = email)} />

				<div class="flex flex-col md:flex-row md:items-start">
					<Button kind="ghost" label="Back to sign in" href={SIGN_IN_ROUTE} />
				</div>
			{/if}
		</div>
	</main>
</div>
