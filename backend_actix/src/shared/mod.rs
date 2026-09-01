#![deny(missing_docs)]

/// The response envelope, the error-code vocabulary, CORS and the JSON
/// extractor config.
pub mod api;
/// Per-caller rate limiting for the unauthenticated auth endpoints.
pub mod rate_limit;
