<script lang="ts">
	import { Button } from '$lib/shared/ui';
	import { resendVerification } from '../api/resend';

	/**
	 * "Resend the link" — the only control on the hold screen.
	 *
	 * Secondary, not primary. This screen has no amber button on purpose: the
	 * gate clears when the emailed link is opened, and a primary action here
	 * would promise the app can move you forward on its own.
	 *
	 * Design: Screen / Verification gate 11:238.
	 */
	let {
		/** The session ended while they were waiting. Nothing to press until they sign in. */
		onsessionexpired
	}: { onsessionexpired?: () => void } = $props();

	let sending = $state(false);
	let outcome = $state<string | undefined>();
	/** Non-null while a rate limit is in force; counts down to zero. */
	let lockedFor = $state<number | null>(null);

	let ticker: ReturnType<typeof setInterval> | undefined;
	$effect(() => () => clearInterval(ticker));

	const locked = $derived(lockedFor !== null && lockedFor > 0);

	function startCountdown(seconds: number) {
		lockedFor = seconds;

		ticker = setInterval(() => {
			const left = (lockedFor ?? 0) - 1;
			lockedFor = left > 0 ? left : null;
			if (left <= 0) {
				outcome = undefined;
				clearInterval(ticker);
			}
		}, 1000);
	}

	async function send() {
		if (sending || locked) return;

		sending = true;
		outcome = undefined;

		const result = await resendVerification();

		sending = false;

		if (result.ok) {
			// Deliberately still pressable: five an hour are allowed, and someone
			// whose mail has not arrived will reasonably try again.
			outcome = result.message;
			return;
		}

		if (result.signedOut) {
			onsessionexpired?.();
			return;
		}

		outcome = result.message;
		if (result.retryAfterSeconds) startCountdown(result.retryAfterSeconds);
	}
</script>

<!--
	One root, so the card's 18px stack sees a single child. A second root would
	put the empty status region in that stack and add a gap below the button
	while saying nothing.
-->
<div class="flex flex-col">
	<!-- Full width on a phone, sized to its label from 768px — Mobile 89:2285
	     puts it edge to edge, and the desktop card sits it on the left. -->
	<div class="flex flex-col self-stretch md:flex-row md:items-start md:self-start">
		<Button
			kind="secondary"
			label="Resend the link"
			disabled={sending || locked}
			disabledReason={locked ? outcome : undefined}
			onclick={send}
		/>
	</div>

	<!--
		Rendered before it has anything to say, and never hidden — a live region
		that is `display: none` while empty leaves the accessibility tree, and a
		change that both fills and reveals it is not reliably announced. The
		margin is conditional instead, so an empty region takes no space.
		Polite: assertive is reserved for loss, and a link being sent has lost
		nothing (Accessibility Spec §09).
	-->
	<p role="status" class="text-[12px] text-arch-muted {outcome ? 'mt-3' : ''}">
		{outcome ?? ''}
	</p>
</div>
