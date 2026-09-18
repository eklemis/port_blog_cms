<script lang="ts">
	/**
	 * The editor rail's Assist card, in the only state this deployment has.
	 *
	 * Screen / AI states 71:2 draws three things the backend can return, and the
	 * first is this one: **assistance is off**. Its annotation is the whole
	 * specification — "503 AI_DISABLED. A deployment with no provider key is a
	 * valid configuration, not a fault — so there is no Retry, because retrying
	 * cannot help. Muted, never red."
	 *
	 * **The working companion is not built, and cannot be yet.** §08 of the Forms
	 * and Interaction Spec describes it: select a passage, a toolbar offers
	 * Tighten · Strengthen · Explain · Ask…, and a suggestion arrives in this rail
	 * with the before and the after and an Apply. Every part of that needs a model
	 * call on the selected text, and the API has four AI endpoints — tailor,
	 * cover-letter, extract-job and quota — none of which takes a passage of prose.
	 * Reported rather than stubbed.
	 *
	 * So the card states the truth: there is no assistance here, nothing is
	 * broken, and there is nothing to press.
	 */
</script>

<section
	aria-label="Assist"
	class="flex flex-col gap-2.5 rounded-xl border border-arch-line bg-arch-surface px-4 py-[15px]"
>
	<h2 class="font-mono text-[9px] font-normal tracking-[0.9px] text-arch-muted uppercase">
		Assist · Off
	</h2>

	<div class="flex flex-col gap-2 rounded-lg bg-arch-surface-2 px-3 py-2.5">
		<p class="text-[12.5px] text-arch-headline">
			Writing assistance is not enabled on this deployment.
		</p>
		<p class="text-[11px] text-arch-muted">
			Everything else works normally. Nothing here is broken.
		</p>
	</div>
</section>
