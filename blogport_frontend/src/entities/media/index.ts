// Public API of the media entity.
export { moved, repositioned } from './model/order';
export { tileStatus, type Tile, type TileStatus } from './model/tile';
export { patchMedia, type MediaChanges, type PatchResult } from './api/patch';
export { archiveMedia, purgeMedia, restoreMedia, type LifecycleResult } from './api/lifecycle';
export {
	beginUpload,
	uploadBytes,
	type AttachmentTarget,
	type Done,
	type Failure,
	type Started,
	type UploadRequest
} from './api/upload';
export {
	coverOf,
	pillFor,
	type Attachment,
	type MediaRole,
	type MediaSize,
	type MediaState
} from './model/media';
export {
	ACCEPTED_TYPES,
	MAX_BYTES,
	MAX_EDGE,
	checkDeclared,
	checkDimensions,
	type Rejection,
	type RejectionCode
} from './model/upload-policy';
