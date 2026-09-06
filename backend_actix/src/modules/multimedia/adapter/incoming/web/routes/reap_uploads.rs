//! The maintenance route a scheduler calls to clear abandoned uploads.
//!
//! # Why this is not authenticated the way everything else is
//!
//! Every other route here belongs to a person and is authorised with their
//! bearer token. This one belongs to no one: Cloud Scheduler calls it on a
//! timer. Cloud Run's own IAM cannot gate it either, because this service is
//! publicly invokable — it serves a public API — so an OIDC identity would be
//! checked by nothing.
//!
//! What is left is a shared secret in a header, which is the arrangement this
//! uses: `X-Maintenance-Token`, compared against `MAINTENANCE_TOKEN`. Two
//! properties make that acceptable rather than merely convenient:
//!
//! - **Unset means gone.** With no `MAINTENANCE_TOKEN` configured the route
//!   answers 404 and does nothing, so a deployment that never sets one has no
//!   destructive endpoint at all rather than an unprotected one.
//! - **Wrong token also means 404**, not 401. A 401 confirms the endpoint
//!   exists and that guessing is worth continuing.
//!
//! A misconfiguration is logged rather than returned, so operators can tell
//! "the token is unset" from "the scheduler sent the wrong one" without either
//! being visible to whoever called.

use actix_web::{post, web, HttpRequest, Responder};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tracing::{error, warn};

use crate::{
    api::schemas::ErrorResponse, multimedia::application::ports::incoming::use_cases::ReapOutcome,
    shared::api::ApiResponse, shared::api::ErrorCode, AppState,
};

/// Optional override for the staleness window.
#[derive(Debug, Default, Deserialize, utoipa::ToSchema)]
pub struct ReapQuery {
    /// How old a registration must be, in seconds, before it is removed.
    ///
    /// Defaults to the deployment's configured window. Values below the signed
    /// upload URL's lifetime are raised to it rather than honoured — see
    /// `ReapStaleUploadsService`.
    pub older_than_secs: Option<u64>,
}

/// Compares two secrets without leaking their contents through timing.
///
/// Both sides are hashed first and the digests compared. A byte-by-byte
/// comparison of the raw values returns sooner the earlier they differ, which
/// is enough to recover a secret one character at a time; digests of different
/// inputs differ everywhere, so where the comparison stops says nothing about
/// the token.
fn secrets_match(presented: &str, expected: &str) -> bool {
    let digest = |s: &str| Sha256::digest(s.as_bytes());

    digest(presented) == digest(expected)
}

/// Clear abandoned uploads
///
/// Deletes media registrations whose bytes never arrived. A row is written when
/// an upload URL is issued, and only the bucket can move it past `pending`; if
/// the client never uploads, the row would otherwise stay forever and any
/// attachment to it would serve an empty `variants` map, which a client cannot
/// distinguish from one still processing.
///
/// Intended for a scheduler. Idempotent — a second call finds nothing and
/// reports zero, which matters because schedulers retry.
#[utoipa::path(
    post,
    path = "/api/maintenance/reap-uploads",
    tag = "maintenance",
    params(
        ("older_than_secs" = Option<u64>, Query,
         description = "Override the staleness window, in seconds. Raised to the \
                        signed upload URL's lifetime if shorter.")
    ),
    responses(
        (status = 200, description = "Sweep ran", body = ReapOutcome),
        (
            status = 404,
            description = "No maintenance token is configured, or the one \
                           presented did not match",
            body = ErrorResponse
        ),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    ),
    security(())
)]
#[post("/api/maintenance/reap-uploads")]
pub async fn reap_uploads_handler(
    req: HttpRequest,
    query: web::Query<ReapQuery>,
    data: web::Data<AppState>,
) -> impl Responder {
    let Some(expected) = std::env::var("MAINTENANCE_TOKEN")
        .ok()
        .filter(|t| !t.trim().is_empty())
    else {
        warn!(
            "A maintenance sweep was requested but MAINTENANCE_TOKEN is unset, \
             so the route is disabled"
        );
        return ApiResponse::not_found(ErrorCode::MediaNotFound, "Not found");
    };

    let presented = req
        .headers()
        .get("X-Maintenance-Token")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();

    if !secrets_match(presented, &expected) {
        warn!("A maintenance sweep was rejected: the token did not match");
        return ApiResponse::not_found(ErrorCode::MediaNotFound, "Not found");
    }

    match data
        .multimedia
        .reap_stale_uploads
        .execute(query.into_inner().older_than_secs)
        .await
    {
        Ok(outcome) => ApiResponse::success(outcome),
        Err(e) => {
            error!("Maintenance sweep failed: {e}");
            ApiResponse::internal_error()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::support::app_state_builder::TestAppStateBuilder;
    // Aliased: importing `test` unqualified shadows the built-in `#[test]`
    // attribute used by the pure-function tests below.
    use actix_web::{http::StatusCode, test as actix_test, App};

    /// The env var is process-wide and these tests disagree about it, so they
    /// take turns. Async-aware because the guard is held across the request.
    static ENV_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

    async fn call(token: Option<&str>, configured: Option<&str>) -> StatusCode {
        let _guard = ENV_LOCK.lock().await;

        match configured {
            Some(v) => std::env::set_var("MAINTENANCE_TOKEN", v),
            None => std::env::remove_var("MAINTENANCE_TOKEN"),
        }

        let state = TestAppStateBuilder::default().build();
        let app =
            actix_test::init_service(App::new().app_data(state).service(reap_uploads_handler))
                .await;

        let mut req = actix_test::TestRequest::post().uri("/api/maintenance/reap-uploads");
        if let Some(t) = token {
            req = req.insert_header(("X-Maintenance-Token", t));
        }

        let status = actix_test::call_service(&app, req.to_request())
            .await
            .status();
        std::env::remove_var("MAINTENANCE_TOKEN");
        status
    }

    #[actix_web::test]
    async fn the_right_token_runs_the_sweep() {
        assert_eq!(call(Some("s3cret"), Some("s3cret")).await, StatusCode::OK);
    }

    /// A deployment that never sets a token has no destructive endpoint at
    /// all, rather than an unprotected one.
    #[actix_web::test]
    async fn an_unconfigured_token_disables_the_route() {
        assert_eq!(call(Some("anything"), None).await, StatusCode::NOT_FOUND);
    }

    /// 404 rather than 401: a 401 confirms the endpoint is real and that
    /// guessing is worth continuing.
    #[actix_web::test]
    async fn a_wrong_token_is_not_told_that_the_route_exists() {
        assert_eq!(
            call(Some("wrong"), Some("s3cret")).await,
            StatusCode::NOT_FOUND
        );
    }

    #[actix_web::test]
    async fn a_missing_header_is_refused() {
        assert_eq!(call(None, Some("s3cret")).await, StatusCode::NOT_FOUND);
    }

    /// An empty configured token must not turn the route into an open one that
    /// any caller with an empty header can trigger.
    #[actix_web::test]
    async fn a_blank_configured_token_does_not_open_the_route() {
        assert_eq!(call(Some(""), Some("   ")).await, StatusCode::NOT_FOUND);
    }

    #[test]
    fn a_matching_token_is_accepted() {
        assert!(secrets_match("s3cret", "s3cret"));
    }

    #[test]
    fn a_different_token_is_rejected() {
        assert!(!secrets_match("s3cret", "other"));
        assert!(!secrets_match("", "s3cret"));
        assert!(!secrets_match("s3cret", ""));
    }

    /// A prefix must not be accepted — the digest comparison is over the whole
    /// value, not a prefix of it.
    #[test]
    fn a_prefix_of_the_token_is_rejected() {
        assert!(!secrets_match("s3cre", "s3cret"));
        assert!(!secrets_match("s3cret", "s3cre"));
    }
}
