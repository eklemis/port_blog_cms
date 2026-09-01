<script lang="ts">
	type User = {
		user_id: string;
		email: string;
		username: string;
		full_name: string;
	};

	let { data } = $props<{
		data: {
			user: User | null;
			error: string | null;
		};
	}>();

	let fullName = $state(data.user?.full_name ?? '');
	let saving = $state(false);
	let successMsg = $state<string | null>(null);
	let errorMsg = $state<string | null>(data.error);

	async function save(event: SubmitEvent) {
		event.preventDefault();

		successMsg = null;
		errorMsg = null;
		saving = true;

		try {
			const res = await fetch('/api/users/me', {
				method: 'PUT',
				headers: { 'content-type': 'application/json' },
				body: JSON.stringify({ full_name: fullName })
			});

			const body = await res.json().catch(() => null);

			if (!res.ok) {
				// backend error shape: { success:false, error:{code,message} }
				const msg =
					body?.error?.message ??
					body?.message ??
					body?.error ??
					`Update failed (${res.status})`;
				throw new Error(msg);
}

			// Update local page state (so UI reflects immediately)
			if (data.user) data.user.full_name = body.full_name ?? fullName;

			successMsg = 'Profile updated.';
		} catch (e) {
			errorMsg = (e as Error).message;
		} finally {
			saving = false;
		}
	}
</script>

<section class="min-h-screen bg-arch-bg text-arch-headline">
	<div class="max-w-3xl mx-auto px-6 py-12 space-y-8">
		<h1 class="text-3xl font-bold">Profile</h1>

		{#if errorMsg}
			<div class="bg-red-500/10 border border-red-500/20 text-red-400 text-sm p-3 rounded-lg" role="alert">
				{errorMsg}
			</div>
		{/if}

		{#if successMsg}
			<div class="bg-emerald-500/10 border border-emerald-500/20 text-emerald-300 text-sm p-3 rounded-lg" role="status">
				{successMsg}
			</div>
		{/if}

		{#if data.user}
			<div class="bg-arch-surface rounded-xl border border-white/5 p-6 space-y-6">
				<div class="space-y-1">
					<div class="text-sm text-arch-muted">Username</div>
					<div class="text-lg font-semibold">{data.user.username}</div>
				</div>

				<div class="space-y-1">
					<div class="text-sm text-arch-muted">Email</div>
					<div class="text-sm">{data.user.email}</div>
				</div>

				<div class="space-y-1">
					<div class="text-sm text-arch-muted">User ID</div>
					<div class="text-xs text-arch-muted">{data.user.user_id}</div>
				</div>

				<form class="pt-4 border-t border-white/5 space-y-4" onsubmit={save}>
					<div class="space-y-2">
						<label for="full_name_input" class="text-sm text-arch-muted">Full Name</label>
						<input
							id="full_name_input"
							type="text"
							required
							bind:value={fullName}
							class="w-full bg-transparent border border-white/10 rounded-lg px-4 py-3 focus:outline-none focus:border-arch-accent"
						/>
					</div>

					<button
						type="submit"
						disabled={saving}
						class="px-4 py-2 rounded-lg bg-arch-accent text-black font-semibold hover:brightness-95 transition disabled:opacity-50"
					>
						{saving ? 'Saving…' : 'Save changes'}
					</button>
				</form>
			</div>
		{:else}
			<div class="bg-arch-surface rounded-xl border border-white/5 p-6 text-arch-muted">
				No profile data.
			</div>
		{/if}
	</div>
</section>
