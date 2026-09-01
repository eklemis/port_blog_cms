<script lang="ts">
	import LoginForm from '$lib/features/auth/ui/login-form.svelte';
	import { resolve } from '$app/paths';
</script>

<section class="min-h-screen flex items-center justify-center bg-arch-bg text-arch-headline px-6">
	<div class="w-full max-w-md space-y-6">
		<LoginForm />

		<p class="text-sm text-center text-arch-muted">
			Don’t have an account?
			<a
				href={resolve('/auth/register')}
				class="text-arch-accent hover:underline"
			>
				Register here
			</a>
		</p>
	</div>
</section>
