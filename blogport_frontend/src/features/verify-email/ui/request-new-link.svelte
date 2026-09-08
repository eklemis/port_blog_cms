<script lang="ts">
	import { Button, Field, InlineAlert } from '$lib/shared/ui';
	import { emailError } from '$lib/shared/lib/email';
	import type { HandlingClass } from '$lib/shared/lib/error-class';
	import { RESEND_SENT_NEUTRAL, resendVerification } from '../api/resend';

	/**
	 * "Send a new link" — the expired-link screen's way out.
	 *
	 * It asks for the address because it has no session to read one from: a
	 * verification link is opened from mail, often on a different device than
	 * the account was created on. The same shape as the password-reset request
	 * in J3, and answered the same way — a confirmation that says nothing about
	 * whether that address has an account.
	 */
	let {
		/** Called with the neutral confirmation once the request is accepted. */
		onsent
	}: { onsent: (message: string) => void } = $props();

	let email = $state('');
	let touched = $state(false);
	let problem = $state<string | undefined>();
	let failure = $state<string | undefined>();
	/** Which of §07's six it was, so the colour is the class's and not a guess. */
	let failureKind = $state<HandlingClass>('notOurs');
	let sending = $state(false);
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

	function startCountdown(seconds: number) {
		lockedFor = seconds;
		ticker = setInterval(() => {
			const left = (lockedFor ?? 0) - 1;
			lockedFor = left > 0 ? left : null;
			if (left <= 0) {
				failure = undefined;
				clearInterval(ticker);
			}
		}, 1000);
	}

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		if (sending || locked) return;

		touched = true;
		problem = emailError(email);
		if (problem) {
			failure = undefined;
			document.getElementById('resend-email')?.focus();
			return;
		}

		sending = true;
		failure = undefined;
		beginWaiting();

		const result = await resendVerification(email);

		sending = false;
		stopWaiting();

		if (result.ok) {
			onsent(RESEND_SENT_NEUTRAL);
			return;
		}

		failure = result.message;
		failureKind = result.kind;
		if (result.retryAfterSeconds) startCountdown(result.retryAfterSeconds);
	}
</script>

<form novalidate onsubmit={submit} class="flex flex-col gap-[18px]">
	<Field
		id="resend-email"
		label="Email"
		type="email"
		name="email"
		autocomplete="email"
		placeholder="jane@example.com"
		required
		bind:value={email}
		error={problem}
		onblur={() => touched && (problem = emailError(email))}
		oninput={() => (touched = true)}
	/>

	<div class="flex flex-col md:flex-row md:items-start">
		<Button type="submit" label="Send a new link" loading={slow} disabled={sending || locked} />
	</div>

	<InlineAlert message={failure} kind={failureKind} />
</form>
