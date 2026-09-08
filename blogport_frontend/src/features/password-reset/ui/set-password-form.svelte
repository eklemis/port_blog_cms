<script lang="ts">
	import { Button, Field, InlineAlert } from '$lib/shared/ui';
	import { PASSWORD_MAX, passwordError } from '$lib/shared/lib/password';
	import type { HandlingClass } from '$lib/shared/lib/error-class';
	import { setPassword } from '../api/password-reset';

	/**
	 * "Set password" — one field with a reveal toggle, not two to mistype in
	 * parallel. Forms Spec §01: the confirm-password twin exists to catch typos
	 * in a masked field, and revealing solves that without doubling the typing.
	 *
	 * Design: Screen / Reset password 67:174 · Mobile 89:2249.
	 */
	let {
		token,
		onset,
		/** The link turned out to be past using; the screen offers a fresh one. */
		onexpired
	}: { token: string; onset: () => void; onexpired: () => void } = $props();

	let password = $state('');
	let touched = $state(false);
	let problem = $state<string | undefined>();
	let failure = $state<string | undefined>();
	/** Which of §07's six it was, so the colour is the class's and not a guess. */
	let failureKind = $state<HandlingClass>('notOurs');
	let sending = $state(false);
	let lockedFor = $state<number | null>(null);

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
		problem = passwordError(password);
		if (problem) {
			failure = undefined;
			document.getElementById('reset-password')?.focus();
			return;
		}

		sending = true;
		failure = undefined;
		spinner = setTimeout(() => (slow = true), 400);

		const result = await setPassword(token, password);

		sending = false;
		clearTimeout(spinner);
		slow = false;

		if (result.ok) {
			onset();
			return;
		}

		if (result.expired) {
			onexpired();
			return;
		}

		// The token stays in the URL and the field keeps its value: losing a
		// valid token to one refused password is a needless restart.
		failure = result.message;
		failureKind = result.kind;
		if (result.retryAfterSeconds) startCountdown(result.retryAfterSeconds);
	}
</script>

<form novalidate onsubmit={submit} class="flex w-full flex-col gap-[18px] md:gap-5">
	<Field
		id="reset-password"
		label="New password"
		type="password"
		name="password"
		autocomplete="new-password"
		maxlength={PASSWORD_MAX}
		required
		bind:value={password}
		help="At least 12 characters. One field, not two — the reveal toggle catches typos."
		error={problem}
		onblur={() => touched && (problem = passwordError(password))}
		oninput={() => (touched = true)}
	/>

	<InlineAlert message={failure} kind={failureKind} />

	<Button
		type="submit"
		label="Set password"
		loading={slow}
		disabled={sending || locked}
		disabledReason={locked ? failure : undefined}
	/>
</form>
