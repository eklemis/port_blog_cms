<script lang="ts">
	import { Menu } from '$lib/shared/ui';
	import { en, type Locale } from '$lib/shared/i18n';

	/**
	 * The "EN ▾" pill — Career Studio §02, and now 30 places rather than 34.
	 *
	 * The frames drew it on every auth screen at all three widths *and* on every
	 * public screen. Ruling A of 20 September took back the 24 public ones:
	 * public pages are not localised, so a switcher there would offer to
	 * translate a shell wrapped around an article the product cannot translate.
	 * What is left is the auth shell and Settings — the places with a person
	 * behind them, reading an interface rather than a document.
	 *
	 * **It does not render while the product speaks one language.** A switcher
	 * over a single catalogue changes a label and nothing else, which is the
	 * defect the Assist card refused to be one screen over. §02 asks that "a
	 * third locale is a file rather than a refactor" — so the day `id.ts` lands,
	 * this appears everywhere it is drawn, with no edit here.
	 *
	 * It shows the language being read, not the one it would switch to: a control
	 * labelled with its own effect reads as a toggle, and this is a menu.
	 */
	let {
		locale,
		offered,
		onchoose = () => {}
	}: {
		locale: Locale;
		/** What the product can actually speak — from `availableLocales()`. */
		offered: readonly Locale[];
		onchoose?: (locale: Locale) => void;
	} = $props();

	/**
	 * Language names are the one thing never translated: a reader looking for
	 * their own language needs to recognise it in that language, not in one they
	 * cannot read.
	 */
	const names = en.language;
</script>

{#if offered.length > 1}
	<Menu label="{names.label}: {locale.toUpperCase()}">
		{#snippet trigger()}
			<span class="flex items-center gap-[5px] text-[10.5px] text-arch-headline">
				{locale.toUpperCase()}
				<span aria-hidden="true" class="text-[9px] text-arch-muted">▾</span>
			</span>
		{/snippet}
		{#snippet items(close)}
			{#each offered as option (option)}
				<button
					type="button"
					role="menuitem"
					aria-current={option === locale ? 'true' : undefined}
					onclick={() => {
						close();
						if (option !== locale) onchoose(option);
					}}
					class="px-4 py-2 text-left text-[13px] text-arch-headline hover:bg-arch-surface-2
					       aria-[current]:font-semibold aria-[current]:text-arch-accent-ink"
				>
					{names[option]}
				</button>
			{/each}
		{/snippet}
	</Menu>
{/if}
