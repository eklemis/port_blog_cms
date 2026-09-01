<script lang="ts">
	import RegisterForm from '$lib/features/auth/ui/register-form.svelte';
	import { resolve } from '$app/paths';
</script>

<section class="min-h-screen flex items-center justify-center bg-arch-bg text-arch-headline px-6">
	<div class="w-full max-w-md space-y-6">
		<RegisterForm />

		<p class="text-sm text-center text-arch-muted">
			Already have an account?
			<a
				href={resolve('/auth/login')}
				class="text-arch-accent hover:underline"
			>
				Login here
			</a>
		</p>
	</div>
</section>
