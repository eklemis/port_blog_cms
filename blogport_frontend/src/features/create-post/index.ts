// Public API of the create-post feature.
export { default as CreatePostForm } from './ui/create-post-form.svelte';
export { createPost, checkSlug, SLUG_TAKEN, type CreateResult } from './api/create-post';
