// src/lib/icons/appui/icons.ts
import IconPhone from './IconPhone.svelte';
import IconEmail from './IconEmail.svelte';
import IconLocation from './IconLocation.svelte';
import IconLink from './IconLink.svelte';
import IconCalendar from './IconCalendar.svelte';

export const icons = {
	phone: IconPhone,
	email: IconEmail,
	location: IconLocation,
	link: IconLink,
	calendar: IconCalendar
} as const;

export type IconName = keyof typeof icons;
