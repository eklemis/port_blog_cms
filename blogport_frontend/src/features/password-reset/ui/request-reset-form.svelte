<script lang="ts">
	import { Button, Field } from '$lib/shared/ui';
	import { emailError } from '$lib/shared/lib/email';
	import { requestReset } from '../api/password-reset';

	/**
	 * "Send reset link" — one address, one button.
	 *
	 * It reports the address it sent for rather than a success message, because
	 * the confirmation must read the same whether or not that address has an
	 * account, and only the screen knows how to say so.
	 */
	let { onsent }: { onsent: (email: string) => void } = $props();

	let email = $state('');
	let touched = $state(false);
	let problem = $state<string | undefined>();
	let failure = $state<string | undefined>();
	let sending = $state(false);
	let lockedFor = $state<number | null>(null);

	/** Suppressed under 400ms, per Forms Spec §04. Sending mail is not fast. */
	let slow = $state(false);
	let spinner: ReturnType<typeof setTimeout> | undefined;
	let ticker: ReturnType<typeof setInterval> | undefined;

	$effect(() => () => {
		clearTimeout(spinner);
		clearInterval(ticker);
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
			document.getElementById('forgot-email')?.focus();
			return;
		}

		sending = true;
		failure = undefined;
		spinner = setTimeout(() => (slow = true), 400);

		const result = await requestReset(email);

		sending = false;
		clearTimeout(spinner);
		slow = false;

		if (result.ok) {
			onsent(email);
			return;
		}

		failure = result.message;
		if (result.retryAfterSeconds) startCountdown(result.retryAfterSeconds);
	}
</script>

<form novalidate onsubmit={submit} class="flex w-full flex-col gap-[18px] md:gap-5">
	<Field
		id="forgot-email"
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

	<p role="status" class="text-[12px] text-st-danger empty:hidden">{failure ?? ''}</p>

	<Button
		type="submit"
		label="Send reset link"
		loading={slow}
		disabled={sending || locked}
		disabledReason={locked ? failure : undefined}
	/>
</form>
