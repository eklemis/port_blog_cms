//! Slug availability, shared by blog and project.
//!
//! Both resources answer the same question the same way, and both index slugs
//! as `(user_id, lower(slug))`, so the suggestion logic lives once rather than
//! twice.

use serde::Serialize;
use utoipa::ToSchema;

/// How many numbered variants to try before giving up.
///
/// Bounded because each candidate is a database round trip. Ten is far past
/// what a human would accept as a suggestion anyway — someone with
/// `my-post-10` already taken does not want `my-post-11`, they want a
/// different title.
const MAX_SUGGESTIONS: u32 = 10;

/// Whether a slug is free, and a free alternative if it is not.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct SlugAvailability {
    /// The slug that was asked about, normalised.
    #[schema(example = "building-a-cms")]
    pub slug: String,

    /// True when the caller can use it.
    #[schema(example = false)]
    pub available: bool,

    /// A free variant, when the requested one is taken.
    ///
    /// `None` when the slug is available, and also when no free variant was
    /// found within the search bound — the caller should then treat the
    /// absence as "pick something else" rather than as an error.
    ///
    /// **The suggestion is checked, not guessed.** Returning an unverified
    /// `-2` would reproduce the problem this endpoint exists to solve: a
    /// collision that only surfaces at save.
    #[schema(example = "building-a-cms-2")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

/// Normalises a slug the way the write paths do, so the answer is about the
/// value that would actually be stored.
pub fn normalize_slug(slug: &str) -> String {
    slug.trim().to_lowercase()
}

/// Finds the first free `{slug}-{n}`, checking each one.
///
/// `taken` is asked about a bounded number of candidates (ten). Any error
/// from it aborts the search and is returned, so a store failure never
/// masquerades as "no suggestion available".
pub async fn suggest_free_slug<F, Fut, E>(base: &str, mut taken: F) -> Result<Option<String>, E>
where
    F: FnMut(String) -> Fut,
    Fut: std::future::Future<Output = Result<bool, E>>,
{
    for n in 2..=(MAX_SUGGESTIONS + 1) {
        let candidate = format!("{base}-{n}");
        if !taken(candidate.clone()).await? {
            return Ok(Some(candidate));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[tokio::test]
    async fn the_first_free_variant_is_suggested() {
        let asked = Mutex::new(Vec::new());
        let s = suggest_free_slug::<_, _, ()>("post", |c| {
            asked.lock().unwrap().push(c.clone());
            async move { Ok(c == "post-2") }
        })
        .await
        .unwrap();

        assert_eq!(s, Some("post-3".to_string()));
        assert_eq!(
            *asked.lock().unwrap(),
            vec!["post-2".to_string(), "post-3".to_string()],
            "it must stop at the first free candidate"
        );
    }

    /// The suggestion is verified, not guessed: if `-2` is taken the answer is
    /// not `-2`. Returning an unchecked variant would reproduce the collision
    /// this endpoint exists to prevent.
    #[tokio::test]
    async fn a_taken_variant_is_never_suggested() {
        let s = suggest_free_slug::<_, _, ()>("post", |c| async move {
            Ok(matches!(c.as_str(), "post-2" | "post-3" | "post-4"))
        })
        .await
        .unwrap();

        assert_eq!(s, Some("post-5".to_string()));
    }

    #[tokio::test]
    async fn the_search_is_bounded() {
        let calls = Mutex::new(0);
        let s = suggest_free_slug::<_, _, ()>("post", |_| {
            *calls.lock().unwrap() += 1;
            async move { Ok(true) }
        })
        .await
        .unwrap();

        assert_eq!(s, None, "no suggestion rather than an unbounded search");
        assert_eq!(*calls.lock().unwrap(), MAX_SUGGESTIONS);
    }

    /// A store failure must surface rather than being reported as "no
    /// suggestion", which would look like a full namespace.
    #[tokio::test]
    async fn a_lookup_failure_aborts_the_search() {
        let result =
            suggest_free_slug::<_, _, &str>("post", |_| async move { Err("db down") }).await;

        assert_eq!(result, Err("db down"));
    }

    #[test]
    fn normalisation_matches_the_write_path() {
        assert_eq!(normalize_slug("  Building-A-CMS  "), "building-a-cms");
    }
}
