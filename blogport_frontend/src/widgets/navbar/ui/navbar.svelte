<script lang="ts">
	import { resolve } from '$app/paths';
	import { invalidateAll } from '$app/navigation';
	import { logout } from '$lib/features/auth/model/auth.api';

	type User =
		| {
				email: string;
				full_name: string;
				user_id: string;
				username: string;
		  }
		| null;

	let { user } = $props<{ user: User }>();

	let loggingOut = $state(false);
	let errorMsg: string | null = $state(null);
	let mobileOpen = $state(false);

	const navLinks = [
		{ label: 'Blogs', href: resolve('/blog') },
		{ label: 'Projects', href: resolve('/projects') },
		{ label: 'Resumes', href: resolve('/resumes') }
	] as const;

	function closeMobile() {
		mobileOpen = false;
	}

	async function handleLogout() {
		errorMsg = null;
		loggingOut = true;

		try {
			await logout();
			closeMobile();
			await invalidateAll();
		} catch (e) {
			errorMsg = (e as Error).message;
		} finally {
			loggingOut = false;
		}
	}

	function onKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') closeMobile();
	}
</script>

<svelte:window onkeydown={onKeydown} />

<header class="relative z-50 bg-arch-bg text-arch-headline border-b border-white/5">
	<nav class="max-w-6xl mx-auto px-6 h-16 flex items-center justify-between">
		<!-- Left -->
		<div class="flex items-center gap-8">
			<a
				href={resolve('/')}
				class="font-extrabold tracking-wide hover:opacity-90"
				onclick={closeMobile}
			>
				Arch
			</a>

			<!-- Desktop links -->
			<div class="hidden md:flex items-center gap-6 text-sm text-arch-muted">
				{#each navLinks as link (link.href)}
					<a class="hover:text-arch-headline transition" href={link.href}>
						{link.label}
					</a>
				{/each}
			</div>
		</div>

		<!-- Right (desktop) -->
		<div class="hidden md:flex items-center gap-3">
			{#if user}
				<a
					href={resolve('/profile')}
					class="px-4 py-2 rounded-lg border border-white/10 text-arch-muted hover:border-white/20 hover:text-arch-headline transition"
				>
					{user.username}
				</a>

				<button
					type="button"
					onclick={handleLogout}
					disabled={loggingOut}
					class="px-4 py-2 rounded-lg bg-arch-surface border border-white/10 hover:border-white/20 transition disabled:opacity-50"
				>
					{loggingOut ? 'Logging out…' : 'Logout'}
				</button>
			{:else}
				<a
					href={resolve('/auth/login')}
					class="px-4 py-2 rounded-lg border border-white/10 text-arch-muted hover:border-white/20 hover:text-arch-headline transition"
				>
					Login
				</a>

				<a
					href={resolve('/auth/register')}
					class="px-4 py-2 rounded-lg bg-arch-accent text-black font-semibold hover:brightness-95 transition"
				>
					Register
				</a>
			{/if}
		</div>

		<!-- Mobile toggle -->
		<button
			type="button"
			class="md:hidden inline-flex items-center justify-center w-10 h-10 rounded-lg border border-white/10 bg-arch-surface hover:border-white/20 transition"
			aria-label="Toggle navigation menu"
			aria-expanded={mobileOpen}
			aria-controls="mobile-nav"
			onclick={() => (mobileOpen = !mobileOpen
)}
		>
			{#if mobileOpen}
				<svg viewBox="0 0 24 24" class="w-5 h-5" fill="none" stroke="currentColor" stroke-width="2">
					<path d="M6 6l12 12M18 6L6 18" />
				</svg>
			{:else}
				<svg viewBox="0 0 24 24" class="w-5 h-5" fill="none" stroke="currentColor" stroke-width="2">
					<path d="M4 6h16M4 12h16M4 18h16" />
				</svg>
			{/if}
		</button>
	</nav>

	{#if errorMsg}
		<div class="max-w-6xl mx-auto px-6 pb-4">
			<div
				class="bg-red-500/10 border border-red-500/20 text-red-400 text-sm p-3 rounded-lg"
				role="alert"
				aria-live="polite"
			>
				{errorMsg}
			</div>
		</div>
	{/if}

	<!-- Mobile panel (FIXED dropdown, cannot be clipped by parent overflow) -->
	{#if mobileOpen}
		<div
			id="mobile-nav"
			class="md:hidden fixed left-0 right-0 top-16 z-50 bg-arch-bg border-b border-white/5 shadow-lg"
		>
			<div class="max-w-6xl mx-auto px-6 py-4 space-y-3">
				<div class="space-y-2">
					{#each navLinks as link (link.href)}
						<a
							class="block px-3 py-2 rounded-lg text-arch-muted hover:text-arch-headline hover:bg-white/5 transition"
							href={link.href}
							onclick={closeMobile}
						>
							{link.label}
						</a>
					{/each}
				</div>

				<div class="pt-2 border-t border-white/5 flex flex-col gap-2">
					{#if user}
						<a
							href={resolve('/profile')}
							class="px-3 py-2 rounded-lg border border-white/10 text-arch-muted hover:border-white/20 hover:text-arch-headline transition"
							onclick={closeMobile}
						>
							Profile ({user.username})
						</a>

						<!-- If your “profile” is actually the public resume page, use this instead:
						<a
							href={resolve({ route: '/resume/[user_name]', params: { user_name: user.username } })}
							class="px-3 py-2 rounded-lg border border-white/10 text-arch-muted hover:border-white/20 hover:text-arch-headline transition"
							onclick={closeMobile}
						>
							Public Resume
						</a>
						-->

						<button
							type="button"
							onclick={handleLogout}
							disabled={loggingOut}
							class="px-3 py-2 rounded-lg bg-arch-surface border border-white/10 hover:border-white/20 transition disabled:opacity-50 text-left"
						>
							{loggingOut ? 'Logging out…' : 'Logout'}
						</button>
					{:else}
						<a
							href={resolve('/auth/login')}
							class="px-3 py-2 rounded-lg border border-white/10 text-arch-muted hover:border-white/20 hover:text-arch-headline transition"
							onclick={closeMobile}
						>
							Login
						</a>

						<a
							href={resolve('/auth/register')}
							class="px-3 py-2 rounded-lg bg-arch-accent text-black font-semibold hover:brightness-95 transition"
							onclick={closeMobile}
						>
							Register
						</a>
					{/if}
				</div>
			</div>
		</div>
	{/if}
</header>
