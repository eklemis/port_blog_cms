// Public API of the sign-in feature. Callers import `$lib/features/sign-in`.
export { default as SignInForm } from './ui/sign-in-form.svelte';
export { destinationAfterSignIn, safeDestination } from './model/gate';
