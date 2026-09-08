<script lang="ts">
	import { tick } from 'svelte';
	import { Button, Field } from '$lib/shared/ui';
	import { emailError } from '$lib/shared/lib/email';
	import { PASSWORD_MAX, passwordError } from '$lib/shared/lib/password';
	import { SIGN_IN_ROUTE } from '$lib/shared/config/routes';
	import {
		FULL_NAME_MAX,
		USERNAME_MAX,
		fullNameError,
		publicAddress,
		usernameError
	} from '../model/fields';
	import { register, type RegisterField } from '../api/register';

	/**
	 * Create your account — username, email, full name, password.
	 *
	 * The branch worth the most is the collision. Someone who already has an
	 * account almost never wants a second one, so 409 marks the email field and
	 * offers the two ways out beside it rather than only refusing.
	 *
	 * Design: Screen / Create account 67:56 · Mobile 89:2136 · Tablet 166:6278.
	 */
	let { onregistered }: { onregistered: () => void } = $props();

	let username = $state('');
	let email = $state('');
	let fullName = $state('');
	let password = $state('');

	// A field validates when it loses focus and only if it has been typed in.
	let touched = $state<Record<RegisterField, boolean>>({
		username: false,
		email: false,
		full_name: false,
		password: false
	});

	let problems = $state<Partial<Record<RegisterField, string | undefined>>>({});
	let failure = $state<string | undefined>();
	let collision = $state(false);
	let submitting = $state(false);
	let lockedFor = $state<number | null>(null);

	/**
	 * Shown only once the request has been slow enough to be worth reporting.
	 * Registration hashes with Argon2 on an unauthenticated request and sends
	 * mail, so it is not fast. Forms Spec §04 keeps the 400ms floor so a quick
	 * one still does not flash.
	 */
	let slow = $state(false);
	let spinner: ReturnType<typeof setTimeout> | undefined;
	let ticker: ReturnType<typeof setInterval> | undefined;

	$effect(() => () => {
		clearTimeout(spinner);
		clearInterval(ticker);
	});

	const locked = $derived(lockedFor !== null && lockedFor > 0);
	const busy = $derived(submitting || locked);

	/** What the username becomes. Permanent, so it is said before submission. */
	const address = $derived(publicAddress(username));
	const usernameHelp = $derived(
		address
			? `Permanent — it becomes your public address, ${address}.`
			: 'Permanent — it becomes your public address.'
	);

	const CHECKS: Record<RegisterField, () => string | undefined> = {
		username: () => usernameError(username),
		email: () => emailError(email),
		full_name: () => fullNameError(fullName),
		password: () => passwordError(password)
	};

	const ORDER: RegisterField[] = ['username', 'email', 'full_name', 'password'];

	function check(field: RegisterField) {
		if (touched[field]) problems = { ...problems, [field]: CHECKS[field]() };
	}

	async function focusFirstInvalid() {
		await tick();
		const first = ORDER.find((field) => problems[field]);
		if (!first) return;

		const input = document.getElementById(`register-${first}`);
		input?.focus();
		input?.scrollIntoView({ block: 'nearest' });
	}

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
		if (busy) return;

		// Submit validates everything, touched or not.
		touched = { username: true, email: true, full_name: true, password: true };
		problems = Object.fromEntries(ORDER.map((field) => [field, CHECKS[field]()]));

		if (ORDER.some((field) => problems[field])) {
			failure = undefined;
			collision = false;
			await focusFirstInvalid();
			return;
		}

		submitting = true;
		failure = undefined;
		collision = false;
		spinner = setTimeout(() => (slow = true), 400);

		const result = await register({ username, email, full_name: fullName, password });

		submitting = false;
		clearTimeout(spinner);
		slow = false;

		if (result.ok) {
			onregistered();
			return;
		}

		// Every value stays. Losing a filled form to one bad field is the worst
		// outcome available.
		collision = result.collision;

		if (result.field) {
			problems = { ...problems, [result.field]: result.message };
			await focusFirstInvalid();
			return;
		}

		failure = result.message;
		if (result.retryAfterSeconds) startCountdown(result.retryAfterSeconds);
	}
</script>

<form novalidate onsubmit={submit} class="flex w-full flex-col gap-[15px] md:gap-5">
	<Field
		id="register-username"
		label="Username"
		name="username"
		autocomplete="username"
		maxlength={USERNAME_MAX}
		required
		bind:value={username}
		help={usernameHelp}
		error={problems.username}
		onblur={() => check('username')}
		oninput={() => (touched = { ...touched, username: true })}
	/>

	<Field
		id="register-email"
		label="Email"
		type="email"
		name="email"
		autocomplete="email"
		required
		bind:value={email}
		error={problems.email}
		onblur={() => check('email')}
		oninput={() => (touched = { ...touched, email: true })}
	/>

	{#if collision}
		<!-- The person almost certainly has an account they forgot, so the two
		     ways out sit with the field that collided rather than at the foot of
		     the form. They pair up side by side even at 390px — Mobile 89:2136. -->
		<div class="flex gap-2">
			<Button kind="secondary" label="Sign in instead" href={SIGN_IN_ROUTE} />
			<Button kind="ghost" label="Reset your password" href="/auth/forgot" />
		</div>
	{/if}

	<Field
		id="register-full_name"
		label="Full name"
		name="full_name"
		autocomplete="name"
		maxlength={FULL_NAME_MAX}
		required
		bind:value={fullName}
		error={problems.full_name}
		onblur={() => check('full_name')}
		oninput={() => (touched = { ...touched, full_name: true })}
	/>

	<Field
		id="register-password"
		label="Password"
		type="password"
		name="password"
		autocomplete="new-password"
		maxlength={PASSWORD_MAX}
		required
		bind:value={password}
		help="At least 12 characters. That is the only rule."
		error={problems.password}
		onblur={() => check('password')}
		oninput={() => (touched = { ...touched, password: true })}
	/>

	<!--
		Always in the DOM so the region exists before it has anything to say.
		Polite: assertive is reserved for loss, and a refused sign-up has lost
		nothing (Accessibility Spec §09).
	-->
	<p role="status" class="text-[12px] text-st-danger empty:hidden">
		{failure ?? ''}
	</p>

	<Button
		type="submit"
		label="Create account"
		loading={slow}
		disabled={busy}
		disabledReason={locked ? failure : undefined}
	/>
</form>
