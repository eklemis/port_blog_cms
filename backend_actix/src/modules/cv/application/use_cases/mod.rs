//! CV use cases.
//!
//! Like `auth`, these predate the `ports/incoming` split: each file declares
//! both the trait and its implementation. See the convention section of
//! `docs/ARCHITECTURE.md`.

/// Creates a CV for the authenticated user.
pub mod create_cv;
/// Fetches one of the caller's own CVs by id.
pub mod fetch_cv_by_id;
/// Lists the caller's CVs.
pub mod fetch_user_cvs;
/// Fetches one CV for a public reader, addressed by username and id.
pub mod get_public_single_cv;
/// Removes a CV permanently.
pub mod hard_delete_cv;
/// Applies a partial update.
pub mod patch_cv;
/// Un-archives a soft-deleted CV.
pub mod restore_cv;
/// Archives a CV without deleting it.
pub mod soft_delete_cv;
/// Replaces a CV's contents wholesale.
pub mod update_cv;
