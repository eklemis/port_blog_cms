// Public API of the edit-cv feature.
export { default as CollectionCard } from './ui/collection-card.svelte';
export { default as ExperienceList } from './ui/experience-list.svelte';
export { patchCv, replaceExperiences, type CvChanges, type SaveResult } from './api/cv';
