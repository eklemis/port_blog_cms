// Public API of the edit-post feature.
export { default as PostEditor } from './ui/post-editor.svelte';
export { createAutosave, type Autosave } from './model/autosave.svelte';
export { patchPost, NO_ACCESS, SLUG_TAKEN, type PostChanges } from './api/update-post';
