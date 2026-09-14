//! One file per resource.

use serde::Deserialize;

use crate::career::application::ports::outgoing::CareerPageRequest;

mod analysis;
mod applications;
mod jobs;
mod letters;

// Glob re-exports carry the hidden `__path_<handler>` structs that
// `#[utoipa::path]` generates alongside each handler; `ApiDoc` needs them.
pub use analysis::*;
pub use applications::*;
pub use jobs::*;
pub use letters::*;

/// Which page of the listing to return.
#[derive(Debug, Default, Deserialize, utoipa::IntoParams)]
pub struct ListPageQuery {
    /// 1-based page number. Omitted or `0` means the first page.
    #[param(example = 1, minimum = 1)]
    #[serde(default)]
    pub page: u32,

    /// Rows per page. Omitted or `0` means 10; anything above 100 is trimmed
    /// to 100 rather than refused.
    #[param(example = 10, minimum = 1, maximum = 100)]
    #[serde(default)]
    pub per_page: u32,
}

impl From<ListPageQuery> for CareerPageRequest {
    fn from(q: ListPageQuery) -> Self {
        Self {
            page: q.page,
            per_page: q.per_page,
        }
    }
}
