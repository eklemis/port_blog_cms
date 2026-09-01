<script lang="ts">
	import { login } from '../model/auth.api';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { invalidateAll } from '$app/navigation';

	let email = $state('');
	let password = $state('');
	let loading = $state(false);
	let errorMsg = $state<string | null>(null);

	async function handleSubmit(event: SubmitEvent) {
		event.preventDefault();

		errorMsg = null;
		loading = true;

		try {
			await login({ email, password });
			await invalidateAll();
			await goto(resolve('/'));
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
	<h2 class="text-2xl font-bold">Login</h2>

	{#if errorMsg}
		<div class="bg-red-500/10 border border-red-500/20 text-red-400 text-sm p-3 rounded-lg">
			{errorMsg}
		</div>
	{/if}

	<div class="space-y-2">
		<label for="email_input" class="text-sm text-arch-muted">Email</label>
		<input
			id="email_input"
			type="email"
			required
			bind:value={email}
			class="w-full bg-transparent border border-white/10 rounded-lg px-4 py-3 focus:outline-none focus:border-arch-accent"
		/>
	</div>

	<div class="space-y-2">
		<label for="password_input" class="text-sm text-arch-muted">Password</label>
		<input
			id="password_input"
			type="password"
			required
			bind:value={password}
			class="w-full bg-transparent border border-white/10 rounded-lg px-4 py-3 focus:outline-none focus:border-arch-accent"
		/>
	</div>

	<button
		type="submit"
		disabled={loading}
		class="w-full bg-arch-accent text-black font-semibold py-3 rounded-lg hover:brightness-95 transition disabled:opacity-50"
	>
		{loading ? 'Signing In...' : 'Login'}
	</button>
</form>
