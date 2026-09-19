<script lang="ts">
	import { Menu as MenuIcon } from '@lucide/svelte';

	/**
	 * The header every public page shares — Screen / Public post 72:3, and
	 * Mobile / Public post 73:293, where the links fold behind ☰.
	 *
	 * Keyed on the author rather than on the page: §03 says the three public
	 * listings share one header so that any entry point leads to the others. It
	 * introduces whoever's work a stranger has landed on, and it offers no way
	 * into the console — "the public reader is not a lapsed author".
	 *
	 * Two things the frame draws that are not here, for two different reasons.
	 *
	 * The "Résumé" link is still waiting: its route in §03 is
	 * `/[username]/cv/[id]`, which cannot be written without knowing which CV an
	 * author nominated.
	 *
	 * The "EN ▾" pill is **not** waiting — it has been ruled out. Ruling A of 20
	 * September: public pages are not localised, so the pill keeps its 30 places
	 * on the auth screens and in Settings and loses its 24 here. It is drawn in
	 * the frames at every public width; do not read the frame and put it back.
	 */
	let {
		username,
		fullName,
		avatarSrc = null,
		current = null,
		measure = 'read'
	}: {
		username: string;
		fullName: string;
		/** A public media path. `null` while it is still being processed, or unset. */
		avatarSrc?: string | null;
		/**
		 * The section being read. It is drawn semibold in accent ink and is not a
		 * link, because a link to the page you are on is a link that does nothing.
		 */
		current?: 'writing' | 'projects' | null;
		/**
		 * Which family of page this sits over. The designer's table: Read — post,
		 * author index, résumé — is 760; Scan — profile, projects, project — is
		 * 860, narrowing to 770 at tablet. The header takes the same measure as
		 * the content beneath it, or it is visibly out of line with it.
		 */
		measure?: 'read' | 'scan';
	} = $props();

	const column = $derived(measure === 'scan' ? 'max-w-[860px]' : 'max-w-[760px]');

	const navId = $props.id();
	const handle = $derived(encodeURIComponent(username));

	let open = $state(false);
</script>

<header
	class="flex items-center justify-center px-[18px] py-[14px] md:h-16 md:px-8 md:py-5"
	data-testid="public-header"
>
	<div class="flex w-full {column} flex-wrap items-center justify-between gap-y-3">
		<div class="flex items-center gap-2 md:gap-[9px]">
			{#if avatarSrc}
				<img
					src={avatarSrc}
					alt={fullName}
					width="24"
					height="24"
					class="size-[22px] rounded-full object-cover md:size-6"
				/>
			{/if}
			<span class="text-[12.5px] font-semibold text-arch-headline md:text-[13px]">{fullName}</span>
		</div>

		<!-- Below md the links fold behind ☰ rather than being drawn twice: two
		     copies of one nav is two things for a screen reader to read out. -->
		<button
			type="button"
			class="text-arch-muted hover:text-arch-headline md:hidden"
			aria-expanded={open}
			aria-controls={navId}
			aria-label="Author navigation"
			onclick={() => (open = !open)}
		>
			<MenuIcon size={15} aria-hidden="true" />
		</button>

		<!-- eslint-disable svelte/no-navigation-without-resolve --
			Both destinations are public screens that do not exist yet, and
			`resolve()` only takes a route id that does. Swap them in as each
			lands. -->
		<nav
			id={navId}
			class="flex items-center gap-[18px] text-[12.5px] text-arch-muted
			       max-md:order-last max-md:w-full max-md:flex-col max-md:items-start max-md:gap-3"
			class:max-md:hidden={!open}
		>
			{#each [{ key: 'writing', label: 'Writing', href: `/${handle}/blog` }, { key: 'projects', label: 'Projects', href: `/${handle}/projects` }] as item (item.key)}
				{#if current === item.key}
					<span aria-current="page" class="font-semibold text-arch-accent-ink">{item.label}</span>
				{:else}
					<a href={item.href} class="hover:text-arch-headline">{item.label}</a>
				{/if}
			{/each}
		</nav>
	</div>
</header>
