<script lang="ts">
	import { Button } from '$lib/shared/ui';
	import { signOut } from '../api/sign-out';

	/**
	 * "Sign out" — a ghost button, because it is the way off a screen rather
	 * than the thing the screen is for.
	 *
	 * It reports rather than navigates: where someone lands after signing out is
	 * the caller's business, and keeping it out of here is what lets this be
	 * reused by the account menu later.
	 */
	let { onsignedout }: { onsignedout?: () => void } = $props();

	let leaving = $state(false);

	async function leave() {
		if (leaving) return;

		leaving = true;
		await signOut();
		// Not reset: the caller is navigating away, and re-enabling the control
		// mid-navigation only invites a second press.
		onsignedout?.();
	}
</script>

<Button kind="ghost" label="Sign out" disabled={leaving} onclick={leave} />
