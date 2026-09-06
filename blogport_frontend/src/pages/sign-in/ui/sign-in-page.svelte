<script lang="ts">
	import { resolve } from '$app/paths';
	import { PRODUCT_NAME } from '$lib/shared/config/product';
	import { SignInForm } from '$lib/features/sign-in';

	/**
	 * Sign in — the whole screen.
	 *
	 * The card is a card from 768px up and a plain full-bleed column below it,
	 * which is the only structural difference between the three widths: the
	 * mobile frame drops the card and the gutters do the work instead.
	 *
	 * Design: Screen / Sign in 67:2 (light) · 67:29 (dark) ·
	 * Tablet / Sign in 131:5161 · Mobile / Sign in 89:2082.
	 * Both themes come from the tokens; there is no second markup path.
	 */
	let {
		next = null,
		sessionExpired = false,
		onsignedin
	}: {
		next?: string | null;
		/** J2 sends people here when a refresh could not save the session. */
		sessionExpired?: boolean;
		onsignedin: (destination: string) => void;
	} = $props();
</script>

<div class="flex min-h-screen flex-col bg-arch-bg">
	<!--
		First focusable element on the page, per Accessibility Spec §05. It skips
		one link today; it belongs in the app layout once the console shell — eight
		links before content — exists to be skipped.
	-->
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
		       md:h-[68px] md:px-[30px] md:py-[22px]"
	>
		<a
			href={resolve('/')}
			class="font-display text-[18px] font-extrabold tracking-tight text-arch-headline md:text-[19px]"
		>
			{PRODUCT_NAME}
		</a>
	</header>

	<main
		id="main"
		class="flex flex-1 flex-col items-center px-5 pt-[26px] pb-12 md:justify-center md:py-10"
	>
		<div
			class="flex w-full max-w-[408px] flex-col gap-[18px] md:gap-5 md:rounded-[14px]
			       md:border md:border-arch-line md:bg-arch-surface md:px-[34px] md:py-[32px]"
		>
			<h1
				class="font-display text-[30px] leading-tight font-extrabold tracking-tight
				       text-arch-headline md:text-[26px]"
			>
				Sign in
			</h1>

			{#if sessionExpired}
				<p class="text-[12.5px] text-arch-muted md:text-[12px]">
					Your session expired. Sign in to pick up where you left off.
				</p>
			{/if}

			<SignInForm {next} {onsignedin} />

			<div class="h-px w-full bg-arch-line" role="presentation"></div>

			<p class="text-[12.5px] text-arch-muted md:text-[12px]">
				New here?
				<!-- /auth/register is in the surface map without being built yet, so
				     there is no route id for resolve() to take. Swap it in when there is. -->
				<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
				<a href="/auth/register" class="font-semibold text-arch-accent-ink hover:underline">
					Create an account
				</a>
			</p>
		</div>
	</main>
</div>
