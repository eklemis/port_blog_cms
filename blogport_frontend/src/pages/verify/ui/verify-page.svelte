<script lang="ts">
	import { resolve } from '$app/paths';
	import { PRODUCT_NAME } from '$lib/shared/config/product';
	import { ResendVerification } from '$lib/features/verify-email';
	import { SignOutButton } from '$lib/features/sign-out';

	/**
	 * The hold screen — where every new account starts.
	 *
	 * `POST /api/auth/login` succeeds for an unverified account and hands back
	 * working tokens; `AuthenticatedUser` lets that account read and edit its own
	 * profile, and `VerifiedUser` guards everything else. So this is not an error
	 * page: it is a legitimate account state, and its job is to be worth looking
	 * at while nothing happens — what the wait is for, where the link went, how
	 * long it lasts, what is locked meanwhile, and a resend.
	 *
	 * Design: Screen / Verification gate 11:238 light · 11:252 dark ·
	 * Tablet 166:5260 · Mobile 89:2285.
	 */
	let {
		/** The address the link went to. Read from the session by the route. */
		email,
		onsessionexpired,
		onsignedout
	}: { email: string; onsessionexpired?: () => void; onsignedout?: () => void } = $props();

	/** What the resend last reported. Rendered below both controls, per the frame. */
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
			<!-- The word carries the meaning; the colour reinforces it. Written in
			     sentence case and uppercased in CSS, so a screen reader reads a
			     phrase rather than spelling out an acronym. -->
			<p
				class="inline-flex self-start rounded-full border border-st-inflight px-2.5 py-1
				       font-mono text-[9px] tracking-[0.9px] text-st-inflight uppercase"
			>
				Awaiting verification
			</p>

			<h1
				class="font-display text-[28px] leading-[34px] font-extrabold tracking-tight
				       text-arch-headline md:text-[26px] md:leading-tight"
			>
				Verify your email to start publishing
			</h1>

			<div
				class="flex flex-col gap-[18px] text-[14.5px] leading-[23px] text-arch-muted
				       md:gap-[22px] md:text-[13.5px] md:leading-[22px]"
			>
				<p class="break-words">We sent a link to {email}. It is good for 24 hours.</p>
				<p>
					Until you open it you can browse your account, but posts, projects, résumés and uploads
					stay locked.
				</p>
			</div>

			<!--
				One card child, so the status region below cannot add an 18px gap to
				the stack while it is empty.
			-->
			<div class="flex flex-col">
				<!-- Side by side from 768px with the frame's 9px gap; stacked and full
				     width on a phone, where a primary action goes edge to edge. -->
				<div class="flex flex-col gap-[18px] md:flex-row md:items-start md:gap-[9px]">
					<ResendVerification {onsessionexpired} onmessage={(m) => (resendMessage = m)} />
					<SignOutButton {onsignedout} />
				</div>

				<!--
					Rendered before it has anything to say, and never hidden — a live
					region that is `display: none` while empty leaves the accessibility
					tree, and a change that both fills and reveals it is not reliably
					announced. Polite: assertive is reserved for loss, and a link being
					sent has lost nothing (Accessibility Spec §09).
				-->
				<p role="status" class="text-[12px] text-arch-muted {resendMessage ? 'mt-3' : ''}">
					{resendMessage ?? ''}
				</p>
			</div>
		</div>
	</main>
</div>
