<script lang="ts">
    import '../app.css';
    import './layout.css';

	import Navbar from '$lib/widgets/navbar/ui/navbar.svelte';

	type User =
		| {
				user_id: string;
				email: string;
				username: string;
				full_name: string;
		  }
		| null;

	let { data, children } = $props<{
		data: { user: User };
		children: any;
	}>();
</script>

<Navbar user={data.user} />
{@render children()}
