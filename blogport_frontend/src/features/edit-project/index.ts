// Public API of the edit-project feature.
export { default as ProjectEditor } from './ui/project-editor.svelte';
export { default as ScreenshotsCard } from './ui/screenshots-card.svelte';
export { attachTopic, detachTopic, patchProject, type ProjectChanges } from './api/project';
