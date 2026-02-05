<script lang="ts">
	import { icons, type IconName } from '$lib/icons/appui/icons';

	type IconSize = 'small' | 'medium' | 'large';

	const {
		name,
		color = 'text-black-100',
		size = 'medium'
	}: {
		name: IconName;
		color?: string;
		size?: IconSize;
	} = $props<{
		name: IconName;
		color?: string;
		size?: IconSize;
	}>();

	const sizeClasses: Record<IconSize, string> = {
		small: 'h-4 w-4',
		medium: 'h-6 w-6',
		large: 'h-8 w-8'
	};

	const iconSizeClass = sizeClasses[size];

	const IconComponent = icons[name];
</script>

<IconComponent class={`${iconSizeClass} ${color}`} />
