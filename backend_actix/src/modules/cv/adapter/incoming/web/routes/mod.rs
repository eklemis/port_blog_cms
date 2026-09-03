mod create_single_cv;
mod cv_snapshots;
mod get_cvs;
mod get_public_single_cv;
mod get_single_cv;
mod hard_delete_single_cv;
mod patch_single_cv;
mod restore_single_cv;
mod soft_delete_single_cv;
mod update_single_cv;

// Glob re-exports matter here: `#[utoipa::path]` generates a hidden
// `__path_<handler>` struct alongside each handler, and `ApiDoc` resolves it
// through this module. Naming a handler explicitly without its `__path_` twin
// fails to compile.
//
// The request DTOs these modules used to duplicate now live in
// `cv::adapter::incoming::web::dto`, so globbing no longer creates ambiguity.
pub use create_single_cv::*;
pub use cv_snapshots::*;
pub use get_cvs::*;
pub use get_public_single_cv::*;
pub use get_single_cv::*;
pub use hard_delete_single_cv::*;
pub use patch_single_cv::*;
pub use restore_single_cv::*;
pub use soft_delete_single_cv::*;
pub use update_single_cv::*;
