<script lang="ts">
	import { resolve } from '$app/paths';

	let { data } = $props<{
		data: {
			ok: boolean;
			message: string;
		};
	}>();
</script>

<section class="min-h-screen flex items-center justify-center bg-arch-bg text-arch-headline px-6">
	<div class="max-w-md w-full bg-arch-surface p-8 rounded-xl border border-white/5 space-y-6">
		<h1 class="text-2xl font-bold">
			{data.ok ? 'Email verified' : 'Verification failed'}
		</h1>

		<p class="text-sm text-arch-muted">
			{data.message}
		</p>

		<div class="pt-2 flex gap-3">
			{#if data.ok}
				<a
					href={resolve('/auth/login')}
					class="px-4 py-2 rounded-lg bg-arch-accent text-black font-semibold hover:brightness-95 transition"
				>
					Go to login
				</a>
			{:else}
				<a
					href={resolve('/auth/register')}
					class="px-4 py-2 rounded-lg border border-white/10 text-arch-muted hover:border-white/20 hover:text-arch-headline transition"
				>
					Register again
				</a>

				<a
					href={resolve('/auth/login')}
					class="px-4 py-2 rounded-lg bg-arch-surface border border-white/10 hover:border-white/20 transition"
				>
					Login
				</a>
			{/if}
		</div>
	</div>
</section>
