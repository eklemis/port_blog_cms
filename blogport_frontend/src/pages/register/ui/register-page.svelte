<script lang="ts">
	import { resolve } from '$app/paths';
	import { PRODUCT_NAME } from '$lib/shared/config/product';
	import { RegisterForm } from '$lib/features/register';

	/**
	 * Create your account — the first screen of J1, and the only one that
	 * crosses both gates.
	 *
	 * Same shell as sign-in and the gate: header at every width, card from
	 * 768px, full-bleed below it.
	 *
	 * Design: Screen / Create account 67:56 · Mobile 89:2136 · Tablet 166:6278.
	 */
	let { onregistered }: { onregistered: () => void } = $props();
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
			class="flex w-full max-w-[408px] flex-col gap-[15px] md:gap-5 md:rounded-[14px]
			       md:border md:border-arch-line md:bg-arch-surface md:px-[34px] md:py-[32px]"
		>
			<h1
				class="font-display text-[28px] leading-[34px] font-extrabold tracking-tight
				       text-arch-headline md:text-[26px] md:leading-tight"
			>
				Create your account
			</h1>

			<RegisterForm {onregistered} />
		</div>
	</main>
</div>
