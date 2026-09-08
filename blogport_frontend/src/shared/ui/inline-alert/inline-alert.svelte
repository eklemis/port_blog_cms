<script lang="ts">
	import { ALERT_TONE, type HandlingClass } from '$lib/shared/lib/error-class';

	/**
	 * The message a form or a region shows when something did not go through.
	 *
	 * Two rules it exists to hold in one place:
	 *
	 * The live region is always in the DOM, and is never `display: none`. One
	 * inserted at the moment of failure is often missed, and an element hidden
	 * while empty has left the accessibility tree — a change that both fills and
	 * reveals it is not reliably announced. Only the colour and the spacing are
	 * conditional.
	 *
	 * And it is `role="status"`, never `role="alert"`. Assertive is reserved for
	 * loss (Accessibility Spec §09); a refusal, a name already taken and a rate
	 * limit have all lost nothing, so none of them earns an interruption.
	 *
	 * Colour comes from the handling class, not from the caller: Console
	 * Blueprint §07 classifies first and writes the sentence second, and red is
	 * spent only where something actually failed.
	 */
	let {
		message,
		kind = 'notOurs',
		class: extra = ''
	}: {
		message?: string;
		/** Which of §07's six this is. Unclassified is treated as their side breaking. */
		kind?: HandlingClass;
		class?: string;
	} = $props();

	const tone = $derived(
		{
			neutral: 'text-arch-muted',
			inflight: 'text-st-inflight',
			danger: 'text-st-danger'
		}[ALERT_TONE[kind]]
	);
</script>

<p role="status" class="text-[12px] {message ? tone : ''} {extra}">
	{message ?? ''}
</p>
