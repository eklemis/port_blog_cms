//! Paging for the career listings.
//!
//! The same four fields the blog, project and CV listings already answer with,
//! so a client writes one paging component rather than one per area. Those
//! three each declare their own copy of this shape; this is a fourth, and the
//! duplication is deliberate — a shared type would have to live somewhere every
//! module may import, and the modules deliberately do not depend on each other.

use serde::Serialize;
use utoipa::ToSchema;

/// Which page to return. Pages are 1-based; defaults to 10 per page.
#[derive(Debug, Clone, Copy)]
pub struct CareerPageRequest {
    /// 1-based page number.
    pub page: u32,
    /// Rows per page.
    pub per_page: u32,
}

impl Default for CareerPageRequest {
    fn default() -> Self {
        Self {
            page: 1,
            per_page: 10,
        }
    }
}

impl CareerPageRequest {
    /// The largest page this will serve.
    ///
    /// A caller asking for more is trimmed rather than refused. The blog
    /// listing clamps the same way and for the same reason: an unbounded page
    /// is a way to ask the database for everything in one request.
    pub const MAX_PER_PAGE: u32 = 100;

    /// Normalises what a client sent into something safe to run.
    ///
    /// `0` means "unset" on both fields — an absent query parameter
    /// deserialises to zero — so it becomes the default rather than an empty
    /// page or a division by zero.
    pub fn normalised(self) -> Self {
        Self {
            page: if self.page == 0 { 1 } else { self.page },
            per_page: match self.per_page {
                0 => 10,
                n => n.min(Self::MAX_PER_PAGE),
            },
        }
    }
}

/// One page of results.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CareerPageResult<T> {
    /// The rows on this page.
    pub items: Vec<T>,

    /// 1-based page number.
    #[schema(example = 1)]
    pub page: u32,

    /// Rows per page, after clamping.
    #[schema(example = 10)]
    pub per_page: u32,

    /// Rows matching across *all* pages, not just this one.
    #[schema(example = 42)]
    pub total: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_absent_page_becomes_the_first_one() {
        let p = CareerPageRequest {
            page: 0,
            per_page: 0,
        }
        .normalised();

        assert_eq!(p.page, 1);
        assert_eq!(p.per_page, 10);
    }

    /// Asking for everything in one request is trimmed, not refused — the same
    /// choice the blog listing makes.
    #[test]
    fn an_enormous_page_is_clamped() {
        let p = CareerPageRequest {
            page: 3,
            per_page: 100_000,
        }
        .normalised();

        assert_eq!(p.per_page, CareerPageRequest::MAX_PER_PAGE);
        assert_eq!(p.page, 3, "the page number itself is left alone");
    }

    #[test]
    fn an_ordinary_request_passes_through() {
        let p = CareerPageRequest {
            page: 2,
            per_page: 25,
        }
        .normalised();

        assert_eq!((p.page, p.per_page), (2, 25));
    }
}
