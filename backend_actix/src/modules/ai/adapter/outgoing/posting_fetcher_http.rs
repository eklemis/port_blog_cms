//! Fetching a job posting over HTTP.
//!
//! Expected to fail most of the time. Most boards block automated fetches or
//! sit behind a login, and the product is designed around that: the link is a
//! shortcut and pasting is the real path. So this tries once, gives a specific
//! reason, and does not retry — the caller is one paste away from succeeding
//! and should not be made to wait.

use async_trait::async_trait;

use crate::ai::application::ports::outgoing::PostingFetcher;

/// The most text worth reading out of one page.
///
/// A posting is a few thousand words; anything past this is navigation and
/// footer, and sending it would pay for tokens that say nothing.
const MAX_CHARS: usize = 40_000;

/// Fetches postings with an ordinary HTTP client.
#[derive(Clone)]
pub struct HttpPostingFetcher {
    http: reqwest::Client,
}

impl HttpPostingFetcher {
    /// Builds it from an HTTP client.
    pub fn new(http: reqwest::Client) -> Self {
        Self { http }
    }
}

#[async_trait]
impl PostingFetcher for HttpPostingFetcher {
    async fn fetch(&self, url: &str) -> Result<String, String> {
        // Only http(s). Without this check a `file://` URL would make this a
        // way to read the server's own filesystem.
        if !(url.starts_with("https://") || url.starts_with("http://")) {
            return Err("Only http and https links can be fetched".to_string());
        }

        let response = self
            .http
            .get(url)
            .send()
            .await
            .map_err(|e| format!("could not reach the page: {e}"))?;

        if !response.status().is_success() {
            return Err(format!(
                "the page answered {} — most job boards block automated fetches",
                response.status()
            ));
        }

        let body = response
            .text()
            .await
            .map_err(|e| format!("could not read the page: {e}"))?;

        // Handed over as fetched, tags and all. Stripping HTML well is its own
        // problem, and a model reads through markup perfectly happily.
        Ok(body.chars().take(MAX_CHARS).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fetcher() -> HttpPostingFetcher {
        HttpPostingFetcher::new(reqwest::Client::new())
    }

    /// Without the scheme check this endpoint would read the server's own
    /// filesystem on request.
    #[tokio::test]
    async fn a_file_url_is_refused_rather_than_read() {
        let err = fetcher().fetch("file:///etc/passwd").await.unwrap_err();

        assert!(err.contains("http"), "{err}");
    }

    #[tokio::test]
    async fn other_schemes_are_refused_too() {
        for url in ["ftp://example.com/job", "gopher://example.com", "job.txt"] {
            assert!(
                fetcher().fetch(url).await.is_err(),
                "{url} should not be fetchable"
            );
        }
    }
}
