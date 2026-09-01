<script lang="ts">
	import { register } from '../model/auth.api';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';

	let full_name = $state('');
	let username = $state('');
	let email = $state('');
	let password = $state('');

	let loading = $state(false);
	let errorMsg = $state<string | null>(null);

	async function handleSubmit(event: SubmitEvent) {
		event.preventDefault();

		errorMsg = null;
		loading = true;

		try {
			await register({
				full_name,
				username,
				email,
				password
			});

			// After successful registration,
			// send user to login page
			await goto(resolve('/auth/login'));
		} catch (e) {
			errorMsg = (e as Error).message;
		} finally {
			loading = false;
		}
	}
</script>

<form
	onsubmit={handleSubmit}
	class="bg-arch-surface p-8 rounded-xl border border-white/5 space-y-6 max-w-md w-full"
>
	<h2 class="text-2xl font-bold">Create Account</h2>

	{#if errorMsg}
		<div
			class="bg-red-500/10 border border-red-500/20 text-red-400 text-sm p-3 rounded-lg"
			role="alert"
			aria-live="polite"
		>
			{errorMsg}
		</div>
	{/if}

	<!-- Full Name -->
	<div class="space-y-2">
		<label for="full_name_input" class="text-sm text-arch-muted">
			Full Name
		</label>
		<input
			id="full_name_input"
			type="text"
			required
			bind:value={full_name}
			class="w-full bg-transparent border border-white/10 rounded-lg px-4 py-3 focus:outline-none focus:border-arch-accent"
		/>
	</div>

	<!-- Username -->
	<div class="space-y-2">
		<label for="username_input" class="text-sm text-arch-muted">
			Username
		</label>
		<input
			id="username_input"
			type="text"
			required
			bind:value={username}
			class="w-full bg-transparent border border-white/10 rounded-lg px-4 py-3 focus:outline-none focus:border-arch-accent"
		/>
	</div>

	<!-- Email -->
	<div class="space-y-2">
		<label for="email_input" class="text-sm text-arch-muted">
			Email
		</label>
		<input
			id="email_input"
			type="email"
			required
			bind:value={email}
			class="w-full bg-transparent border border-white/10 rounded-lg px-4 py-3 focus:outline-none focus:border-arch-accent"
		/>
	</div>

	<!-- Password -->
	<div class="space-y-2">
		<label for="password_input" class="text-sm text-arch-muted">
			Password
		</label>
		<input
			id="password_input"
			type="password"
			required
			minlength="8"
			bind:value={password}
			class="w-full bg-transparent border border-white/10 rounded-lg px-4 py-3 focus:outline-none focus:border-arch-accent"
		/>
		<p class="text-xs text-arch-muted">
			Minimum 8 characters.
		</p>
	</div>

	<button
		type="submit"
		disabled={loading}
		class="w-full bg-arch-accent text-black font-semibold py-3 rounded-lg hover:brightness-95 transition disabled:opacity-50"
	>
		{loading ? 'Creating Account...' : 'Register'}
	</button>
</form>
