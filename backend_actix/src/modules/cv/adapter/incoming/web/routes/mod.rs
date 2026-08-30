mod create_single_cv;
mod get_cvs;
mod get_public_single_cv;
mod get_single_cv;
mod hard_delete_single_cv;
mod patch_single_cv;
mod update_single_cv;

// Glob re-exports matter here: `#[utoipa::path]` generates a hidden
// `__path_<handler>` struct alongside each handler, and `ApiDoc` resolves it
// through this module. Naming a handler explicitly without its `__path_` twin
// fails to compile.
pub use create_single_cv::*;
pub use get_cvs::*;
pub use get_public_single_cv::*;
pub use get_single_cv::*;
pub use hard_delete_single_cv::*;
pub use patch_single_cv::*;

// `update_single_cv` re-declares three request DTOs that `create_single_cv`
// already exports, so it cannot be globbed without making those names
// ambiguous. The aliases match the `#[schema(as = ...)]` names those types
// carry in the OpenAPI document.
pub use update_single_cv::{
    __path_update_cv_handler, update_cv_handler, ContactDetailRequest,
    EducationRequest as UpdateEducationRequest, ExperienceRequest as UpdateExperienceRequest,
    HighlightedProjectRequest as UpdateHighlightedProjectRequest, UpdateCVRequest,
};
