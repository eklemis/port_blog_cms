<script lang="ts">
	/**
	 * StatusPill — five values, one meaning each, identical across every
	 * resource. A scheduled post and a processing upload are the same tone,
	 * because they are the same kind of fact: something in flight.
	 *
	 * Named after the tone rather than the resource for exactly that reason —
	 * the word in `label` is what varies, and it is what carries the meaning.
	 * Status colour never appears on a button or a link.
	 *
	 * Design: StatusPill 19:39. Accessibility Spec §08: a `<span>`, nothing
	 * extra — a pill in a table is not an announcement.
	 */
	type Tone = 'neutral' | 'inflight' | 'live' | 'dormant' | 'danger';

	let { tone, label }: { tone: Tone; label: string } = $props();

	const TONE: Record<Tone, string> = {
		neutral: 'border-st-neutral text-st-neutral',
		inflight: 'border-st-inflight text-st-inflight',
		live: 'border-st-live text-st-live',
		dormant: 'border-st-dormant text-st-dormant',
		danger: 'border-st-danger text-st-danger'
	};
</script>

<span
	class="inline-flex items-center gap-1.5 rounded-full border px-2.5 py-[3px] font-mono
	       text-[9px] tracking-[0.45px] uppercase {TONE[tone]}"
>
	<!-- The dot repeats the colour; the word is what actually says it. -->
	<span class="size-1.5 rounded-full bg-current" aria-hidden="true"></span>
	{label}
</span>
