<script lang="ts">
	import { tick } from 'svelte';
	import { Button, Field } from '$lib/shared/ui';
	import { emailError } from '$lib/shared/lib/email';
	import { PASSWORD_MAX, passwordError } from '../model/credentials';
	import { destinationAfterSignIn } from '../model/gate';
	import { signIn } from '../api/sign-in';

	/**
	 * Sign in — email, password, one amber button.
	 *
	 * The form does not navigate; it reports where the person should go and lets
	 * the page route. That keeps the gate — the `is_verified` decision in
	 * ../model/gate — a pure function this spec can hold to account.
	 *
	 * Design: Figma Screen / Sign in 67:2 (light) · 67:29 (dark) ·
	 * Tablet 131:5161 · Mobile 89:2082. Behaviour: Forms & Interaction Spec §05.
	 */
	let {
		/** A destination saved before a session expired. Ignored for an unverified account. */
		next = null,
		onsignedin
	}: {
		next?: string | null;
		onsignedin: (destination: string) => void;
	} = $props();

	let email = $state('');
	let password = $state('');

	// A field validates when it loses focus and only if the person has typed in
	// it. An untouched empty required field is not an error until submit.
	let emailTouched = $state(false);
	let passwordTouched = $state(false);

	let emailProblem = $state<string | undefined>();
	let passwordProblem = $state<string | undefined>();
	let failure = $state<string | undefined>();

	let submitting = $state(false);
	/** Non-null while a rate limit is in force; counts down to zero. */
	let lockedFor = $state<number | null>(null);

	const locked = $derived(lockedFor !== null && lockedFor > 0);
	const busy = $derived(submitting || locked);

	function checkEmail() {
		if (emailTouched) emailProblem = emailError(email);
	}

	function checkPassword() {
		if (passwordTouched) passwordProblem = passwordError(password);
	}

	async function focusFirstInvalid() {
		await tick();
		const id = emailProblem ? 'signin-email' : passwordProblem ? 'signin-password' : null;
		if (!id) return;

		const field = document.getElementById(id);
		field?.focus();
		field?.scrollIntoView({ block: 'nearest' });
	}

	function startCountdown(seconds: number) {
		lockedFor = seconds;

		const tick = setInterval(() => {
			const left = (lockedFor ?? 0) - 1;
			lockedFor = left > 0 ? left : null;
			failure = left > 0 ? failure : undefined;
			if (left <= 0) clearInterval(tick);
		}, 1000);
	}

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		if (busy) return;

		// Submit validates everything, whether or not it was touched.
		emailTouched = true;
		passwordTouched = true;
		emailProblem = emailError(email);
		passwordProblem = passwordError(password);

		if (emailProblem || passwordProblem) {
			failure = undefined;
			await focusFirstInvalid();
			return;
		}

		submitting = true;
		failure = undefined;

		const result = await signIn({ email, password });

		submitting = false;

		if (result.ok) {
			onsignedin(destinationAfterSignIn(result.user, next));
			return;
		}

		// Every value stays: losing a filled form to one bad field is the worst
		// outcome available.
		failure = result.message;
		if (result.retryAfterSeconds) startCountdown(result.retryAfterSeconds);
	}
</script>

<form novalidate onsubmit={submit} class="flex w-full flex-col gap-[18px] md:gap-5">
	<Field
		id="signin-email"
		label="Email"
		type="email"
		name="email"
		autocomplete="username"
		placeholder="jane@example.com"
		required
		bind:value={email}
		error={emailProblem}
		onblur={checkEmail}
		oninput={() => (emailTouched = true)}
	/>

	<Field
		id="signin-password"
		label="Password"
		type="password"
		name="password"
		autocomplete="current-password"
		maxlength={PASSWORD_MAX}
		required
		bind:value={password}
		error={passwordProblem}
		onblur={checkPassword}
		oninput={() => (passwordTouched = true)}
	>
		<!-- eslint-disable svelte/no-navigation-without-resolve --
			`resolve()` only takes a route id that exists, and /auth/forgot is in the
			surface map without being built yet. Swap it in when it is. -->
		{#snippet labelAction()}
			<!--
				`min-h-6` is not decoration: an 11.5px inline link is about 14px
				tall, and WCAG 2.2 AA 2.5.8 wants 24. The audit's table sized every
				control but this one — see the PR. It grows the 16px label row to
				24px, which is the whole visual cost.
			-->
			<a
				href="/auth/forgot"
				class="inline-flex min-h-6 items-center text-arch-accent-ink hover:underline"
			>
				Forgot password?
			</a>
		{/snippet}
	</Field>

	<!--
		Always in the DOM so the region exists before it has anything to say —
		one inserted at the moment of failure is often missed. Polite, not
		assertive: assertive is reserved for loss (Accessibility Spec §09), and
		a sign-in that did not go through has lost nothing.
	-->
	<p role="status" class="text-[12px] text-st-danger empty:hidden">
		{failure ?? ''}
	</p>

	<Button
		type="submit"
		label="Sign in"
		disabled={busy}
		disabledReason={locked ? failure : undefined}
	/>
</form>
