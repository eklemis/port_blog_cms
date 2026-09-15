<script lang="ts">
	import type { Snippet } from 'svelte';
	import { ChevronLeft, Ellipsis } from '@lucide/svelte';
	import { Button } from '$lib/shared/ui';
	import { resolve } from '$app/paths';
	import { PRODUCT_NAME } from '$lib/shared/config/product';
	import { ACCOUNT, MORE, NAV, TAB_BAR, currentLabel, isCurrent, mobileChrome } from '../model/nav';

	/**
	 * The console's frame: the same navigation at three widths.
	 *
	 * A 212px sidebar from 1024px, a 64px icon rail between 768 and 1023, and a
	 * five-slot tab bar below that — four destinations and More — reading one
	 * nav definition so the three shapes cannot drift apart.
	 *
	 * Design: Screen / Overview 68:2 · Mobile / Overview 90:2188 · Mobile / More
	 * 197:5098. The mobile header is the sidebar's stand-in: the section's name
	 * and its one action, or a way back inside a section — see `mobileChrome`.
	 */
	let {
		/** The current path, so the right item is lit and the header is right. */
		path,
		children
	}: { path: string; children: Snippet } = $props();

	const chrome = $derived(mobileChrome(path));

	// In the script rather than the markup: `{@const}` is only legal as the
	// immediate child of a block, and this one sits inside an element.
	const accountCurrent = $derived(isCurrent(ACCOUNT.href, path));

	/**
	 * A native `<dialog>`, opened modally: the focus trap, Escape and the return
	 * of focus to whatever opened it are all the platform's, and all three are
	 * §06 requirements that hand-rolled sheets routinely get wrong.
	 */
	let sheet = $state<HTMLDialogElement>();

	/** Closed on navigation too — a client-side route change leaves it standing. */
	function closeMore() {
		sheet?.close();
	}
</script>

<!-- eslint-disable svelte/no-navigation-without-resolve --
	The nav's destinations come from shared/config/routes, and every one of them
	is a console screen that does not exist yet — `resolve()` only takes a route
	id that does. Swap them in as each lands. -->
<div class="flex min-h-screen flex-col bg-arch-bg md:flex-row">
	<a
		href="#main"
		class="sr-only focus:not-sr-only focus:absolute focus:top-2 focus:left-2 focus:z-20
		       focus:rounded-lg focus:border focus:border-arch-line-control focus:bg-arch-surface
		       focus:px-3 focus:py-2 focus:text-[13px] focus:text-arch-headline"
	>
		Skip to content
	</a>

	<!-- Mobile: the sidebar has no room, so the screen names itself instead. -->
	{#if !chrome.bare}
		<header
			class="flex h-[58px] shrink-0 items-center justify-between gap-3 bg-arch-surface px-[18px]
		       py-3.5 md:hidden"
		>
			<div class="flex items-center gap-2">
				{#if chrome.back}
					<!-- A 32px target around a 19px chevron, named for where it goes. -->
					<a
						href={chrome.back}
						aria-label="Back to {currentLabel(chrome.back)}"
						class="-ml-2 flex size-8 items-center justify-center text-arch-headline"
					>
						<ChevronLeft size={19} aria-hidden="true" />
					</a>
				{/if}
				<!-- Not a heading: each screen keeps its own h1, visually hidden on a
			     phone, so there is exactly one at every width. -->
				<p class="font-display text-[18px] font-bold text-arch-headline">{chrome.title}</p>
			</div>
			{#if chrome.action}
				<Button label={chrome.action.label} href={chrome.action.href} />
			{/if}
		</header>
	{/if}

	<!-- Sidebar from 768px: icons only until 1024, then labels too. -->
	<nav
		aria-label="Console"
		class="hidden w-16 shrink-0 flex-col gap-1 bg-arch-surface px-2 py-5 md:flex
		       lg:w-[212px] lg:px-3.5"
	>
		<a
			href={resolve('/')}
			class="mb-2 px-2.5 py-1 font-display text-[19px] font-extrabold tracking-tight
			       text-arch-headline max-lg:sr-only"
		>
			{PRODUCT_NAME}
		</a>

		{#each NAV as item (item.href)}
			{@const current = isCurrent(item.href, path)}
			<a
				href={item.href}
				aria-current={current ? 'page' : undefined}
				title={item.label}
				class="flex h-[34px] items-center gap-2.5 rounded-[7px] px-2.5 text-[13px]
				       max-lg:justify-center
				       {current
					? 'bg-arch-surface-2 font-semibold text-arch-accent-ink'
					: 'text-arch-muted hover:bg-arch-surface-2'}"
			>
				<item.icon size={17} aria-hidden="true" />
				<span class="max-lg:sr-only">{item.label}</span>
			</a>
		{/each}

		<!-- Screen / Overview 68:21: a 20px gap, then the rule, then Account. -->
		<div class="mt-5 h-px bg-arch-line" role="presentation"></div>

		<a
			href={ACCOUNT.href}
			aria-current={accountCurrent ? 'page' : undefined}
			title={ACCOUNT.label}
			class="flex h-[34px] items-center gap-2.5 rounded-[7px] px-2.5 text-[13px]
			       max-lg:justify-center
			       {accountCurrent
				? 'bg-arch-surface-2 font-semibold text-arch-accent-ink'
				: 'text-arch-muted hover:bg-arch-surface-2'}"
		>
			<ACCOUNT.icon size={17} aria-hidden="true" />
			<span class="max-lg:sr-only">{ACCOUNT.label}</span>
		</a>
	</nav>

	<main
		id="main"
		class="flex-1 p-4 md:px-[30px] md:py-[26px] {chrome.tabs ? 'max-md:pb-[84px]' : ''}"
	>
		{@render children()}
	</main>

	{#if chrome.tabs}
		<!--
		Below 768px: five destinations, everything else behind More. A wrapped
		tab row pushes the content below the fold, which defeats the screen.
	-->
		<nav
			data-tab-bar
			aria-label="Console"
			class="fixed inset-x-0 bottom-0 flex h-[68px] items-stretch bg-arch-surface px-2 py-2.5
		       md:hidden"
		>
			{#each TAB_BAR as item (item.label)}
				{@const current = isCurrent(item.href, path)}
				<a
					href={item.href}
					aria-current={current ? 'page' : undefined}
					class="flex flex-1 flex-col items-center justify-center gap-1.5 rounded-lg
				       text-[10px]
				       {current ? 'font-semibold text-arch-accent-ink' : 'text-arch-muted'}"
				>
					<item.icon size={21} aria-hidden="true" />
					<!-- The short name where there is one: "Apps" is what the frame
				     prints, and it is the accessible name too. -->
					{item.short ?? item.label}
				</a>
			{/each}

			<!--
			The fifth slot is not a destination. It was a link to Account until the
			mobile transform table was written down, which left Overview, Résumés
			and Topics with no way in at 390px at all.
		-->
			<button
				type="button"
				onclick={() => sheet?.showModal()}
				class="flex flex-1 flex-col items-center justify-center gap-1.5 rounded-lg text-[10px]
			       text-arch-muted"
			>
				<Ellipsis size={21} aria-hidden="true" />
				More
			</button>
		</nav>
	{/if}

	<!-- Mobile / More 197:5098: a grip and four 52px rows over the scrim. No
	     heading is drawn, so the dialog is named by its label. -->
	<dialog
		bind:this={sheet}
		aria-modal="true"
		aria-label="More"
		onclick={(event) => {
			// The scrim is the dialog's own box outside its content, so a click
			// that lands on the element itself landed on the scrim.
			if (event.target === sheet) closeMore();
		}}
		class="m-0 mt-auto w-full max-w-none rounded-t-2xl bg-arch-surface pt-2.5 pb-[18px]
		       backdrop:bg-arch-scrim md:hidden"
	>
		<div class="flex h-5 items-center justify-center" aria-hidden="true">
			<div class="h-1 w-9 rounded-sm bg-arch-line"></div>
		</div>

		<nav aria-label="More">
			{#each MORE as item (item.href)}
				{@const current = isCurrent(item.href, path)}
				<a
					href={item.href}
					aria-current={current ? 'page' : undefined}
					onclick={closeMore}
					class="flex h-[52px] items-center gap-3.5 px-5 text-[15px] font-medium
					       {current ? 'text-arch-accent-ink' : 'text-arch-headline'}"
				>
					<item.icon size={20} aria-hidden="true" />
					{item.label}
				</a>
			{/each}
		</nav>
	</dialog>
</div>
