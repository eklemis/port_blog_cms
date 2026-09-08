<script lang="ts">
	import type { Snippet } from 'svelte';
	import { resolve } from '$app/paths';
	import { PRODUCT_NAME } from '$lib/shared/config/product';
	import { ACCOUNT, NAV, TAB_BAR, isCurrent } from '../model/nav';

	/**
	 * The console's frame: the same navigation at three widths.
	 *
	 * A 212px sidebar from 1024px, a 64px icon rail between 768 and 1023, and a
	 * five-item tab bar below that with everything else behind More — the three
	 * shapes the responsive section describes, reading one nav definition so
	 * they cannot drift apart.
	 *
	 * Design: Screen / Overview 68:2 · Mobile / Overview 90:2188.
	 */
	let {
		/** The current path, so the right item is lit. */
		path,
		/** Named on the mobile bar, where there is no room for the sidebar to say it. */
		title,
		children
	}: { path: string; title: string; children: Snippet } = $props();

	// In the script rather than the markup: `{@const}` is only legal as the
	// immediate child of a block, and this one sits inside an element.
	const accountCurrent = $derived(isCurrent(ACCOUNT.href, path));
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
	<header class="flex h-[58px] shrink-0 items-center bg-arch-surface px-[18px] py-3.5 md:hidden">
		<p class="font-display text-[18px] font-bold text-arch-headline">{title}</p>
	</header>

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

		<div class="my-2 h-px bg-arch-line" role="presentation"></div>

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

	<main id="main" class="flex-1 p-4 max-md:pb-[84px] md:px-[30px] md:py-[26px]">
		{@render children()}
	</main>

	<!--
		Below 768px: five destinations, everything else behind More. A wrapped
		tab row pushes the content below the fold, which defeats the screen.
	-->
	<nav
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
				{item.label}
			</a>
		{/each}
	</nav>
</div>
