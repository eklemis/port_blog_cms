<script lang="ts">
	import { Button } from '$lib/shared/ui';
	import { RESEND_SENT, resendVerification } from '../api/resend';

	/**
	 * "Resend the link" — secondary, not primary. The gate screen has no amber
	 * button on purpose: it clears when the emailed link is opened, and a
	 * primary action would promise the app can move you forward on its own.
	 *
	 * It renders the control only and reports its outcome upward, because the
	 * frame puts a second button beside it and the message below both — layout
	 * the screen owns, not this.
	 *
	 * Design: Screen / Verification gate 11:238.
	 */
	let {
		/** The session ended while they were waiting. Nothing to press until they sign in. */
		onsessionexpired,
		/** What to say about the last attempt, or undefined once a lock expires. */
		onmessage
	}: { onsessionexpired?: () => void; onmessage?: (message: string | undefined) => void } =
		$props();

	let sending = $state(false);
	let outcome = $state<string | undefined>();
	/** Non-null while a rate limit is in force; counts down to zero. */
	let lockedFor = $state<number | null>(null);

	/**
	 * Shown only once the request has been slow enough to be worth reporting.
	 * Forms & Interaction Spec §04: suppressed under 400ms, because a spinner
	 * that flashes for one frame reads as a glitch rather than as progress.
	 * Resending is not fast — the server sends mail before it answers.
	 */
	let slow = $state(false);
	let spinner: ReturnType<typeof setTimeout> | undefined;

	function beginWaiting() {
		spinner = setTimeout(() => (slow = true), 400);
	}

	function stopWaiting() {
		clearTimeout(spinner);
		slow = false;
	}

	let ticker: ReturnType<typeof setInterval> | undefined;
	$effect(() => () => {
		clearInterval(ticker);
		clearTimeout(spinner);
	});

	const locked = $derived(lockedFor !== null && lockedFor > 0);

	function report(message: string | undefined) {
		outcome = message;
		onmessage?.(message);
	}

	function startCountdown(seconds: number) {
		lockedFor = seconds;

		ticker = setInterval(() => {
			const left = (lockedFor ?? 0) - 1;
			lockedFor = left > 0 ? left : null;
			if (left <= 0) {
				report(undefined);
				clearInterval(ticker);
			}
		}, 1000);
	}

	async function send() {
		if (sending || locked) return;

		sending = true;
		report(undefined);
		beginWaiting();

		const result = await resendVerification();

		sending = false;
		stopWaiting();

		if (result.ok) {
			// Deliberately still pressable: five an hour are allowed, and someone
			// whose mail has not arrived will reasonably try again.
			report(RESEND_SENT);
			return;
		}

		if (result.signedOut) {
			onsessionexpired?.();
			return;
		}

		report(result.message);
		if (result.retryAfterSeconds) startCountdown(result.retryAfterSeconds);
	}
</script>

<Button
	kind="secondary"
	label="Resend the link"
	loading={slow}
	disabled={sending || locked}
	disabledReason={locked ? outcome : undefined}
	onclick={send}
/>
