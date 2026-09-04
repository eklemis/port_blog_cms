//! The three generation surfaces.
//!
//! Two of them stream. That is not decoration: a tailoring pass or a cover
//! letter takes long enough that a modal spinner over the wait is the
//! difference between a tool and a queue — and long enough to hit a proxy
//! timeout before producing anything at all.

use actix_web::{post, web, HttpResponse, Responder};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use tracing::{error, warn};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    ai::application::ports::incoming::use_cases::{
        AiError, DraftingInput, ExtractJobInput, ExtractedJob,
    },
    ai::application::ports::outgoing::{GenerationEvent, GenerationStream},
    api::schemas::{ErrorResponse, SuccessResponse},
    auth::{
        adapter::incoming::web::extractors::auth::VerifiedUser,
        application::domain::entities::UserId,
    },
    shared::api::{ApiResponse, ErrorCode},
    AppState,
};

/// Where to read a job posting from.
#[derive(Debug, Default, Deserialize, ToSchema)]
pub struct ExtractJobRequest {
    /// The posting, pasted. **The primary path.**
    pub text: Option<String>,

    /// A link to the posting.
    ///
    /// A shortcut, and one that usually fails — most boards block automated
    /// fetches or sit behind a login. When it does, the answer is
    /// `AI_FETCH_FAILED` and the remedy is to paste. It is not retried.
    pub url: Option<String>,
}

/// What a drafting pass should work from.
#[derive(Debug, Default, Deserialize, ToSchema)]
pub struct DraftingRequest {
    /// The application being worked on.
    pub application_id: Uuid,

    /// A living CV to work from. Falls back to the application's snapshot;
    /// a draft with neither is a 400.
    pub cv_id: Option<Uuid>,

    /// What to do this turn, in your own words. Each surface has a default.
    pub instruction: Option<String>,

    /// The language to write in. Explicit, never inferred.
    pub language: Option<String>,
}

fn map_error(e: AiError) -> HttpResponse {
    match e {
        AiError::Disabled => ApiResponse::error(
            actix_web::http::StatusCode::SERVICE_UNAVAILABLE,
            ErrorCode::AiDisabled,
            "Generation is not configured on this deployment",
        ),
        AiError::QuotaExceeded(state) => ApiResponse::error(
            actix_web::http::StatusCode::TOO_MANY_REQUESTS,
            ErrorCode::AiQuotaExceeded,
            &format!(
                "Generation limit reached. It resets at {}.",
                state.resets_at.to_rfc3339()
            ),
        ),
        AiError::FetchFailed(detail) => ApiResponse::error(
            actix_web::http::StatusCode::UNPROCESSABLE_ENTITY,
            ErrorCode::AiFetchFailed,
            &detail,
        ),
        AiError::Refused(detail) => ApiResponse::error(
            actix_web::http::StatusCode::UNPROCESSABLE_ENTITY,
            ErrorCode::AiRefused,
            &detail,
        ),
        AiError::Timeout => ApiResponse::error(
            actix_web::http::StatusCode::GATEWAY_TIMEOUT,
            ErrorCode::AiTimeout,
            "The model took too long to answer",
        ),
        AiError::Invalid(detail) => ApiResponse::bad_request(ErrorCode::ValidationError, &detail),
        AiError::Upstream(detail) => {
            // Logged rather than returned: a provider's message can carry
            // prompt fragments, and a prompt here contains somebody's CV.
            error!("Generation provider failed: {}", detail);
            ApiResponse::error(
                actix_web::http::StatusCode::BAD_GATEWAY,
                ErrorCode::AiUpstreamError,
                "The model provider failed. This is usually worth retrying.",
            )
        }
    }
}

/// One frame of a streamed generation, as the client sees it.
#[derive(Debug, Serialize, ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamFrame {
    /// More text. Append it; it is not cumulative.
    Delta {
        /// The new text.
        text: String,
    },
    /// The generation finished.
    Done {
        /// Tokens the provider billed for, for cost attribution.
        input_tokens: u32,
        /// Tokens generated.
        output_tokens: u32,
        /// Tokens served from the provider's cache. **Zero on a repeated call
        /// means prefix caching has stopped working**, which is worth
        /// noticing — it is the largest cost lever in the feature.
        cached_input_tokens: u32,
    },
    /// Something went wrong. May arrive **after** text already has.
    Error {
        /// The same vocabulary the non-streamed routes use.
        code: String,
        /// One sentence, safe to show.
        message: String,
    },
}

/// Turns the port's events into the frames a browser reads.
///
/// Errors become frames rather than ending the response, because by the time
/// one arrives the status line is long gone — a 200 has already been sent and
/// text may already be on screen. A stream that simply stopped would look to a
/// client exactly like a finished generation.
fn to_frames(
    stream: GenerationStream,
) -> impl futures::Stream<Item = Result<web::Bytes, actix_web::Error>> {
    stream.map(|event| {
        let frame = match event {
            Ok(GenerationEvent::Delta(text)) => StreamFrame::Delta { text },
            Ok(GenerationEvent::Completed(usage)) => StreamFrame::Done {
                input_tokens: usage.input_tokens,
                output_tokens: usage.output_tokens,
                cached_input_tokens: usage.cached_input_tokens,
            },
            Err(e) => {
                let (code, message) = match AiError::from(e) {
                    AiError::Refused(d) => (ErrorCode::AiRefused, d),
                    AiError::Timeout => (
                        ErrorCode::AiTimeout,
                        "The model took too long to answer".to_string(),
                    ),
                    other => {
                        warn!("Generation stream failed: {}", other);
                        (
                            ErrorCode::AiUpstreamError,
                            "The model provider failed part-way through.".to_string(),
                        )
                    }
                };
                StreamFrame::Error {
                    code: code.as_str().to_string(),
                    message,
                }
            }
        };

        let line = serde_json::to_string(&frame).unwrap_or_else(|_| {
            r#"{"type":"error","code":"AI_UPSTREAM_ERROR","message":"unreadable frame"}"#
                .to_string()
        });

        Ok(web::Bytes::from(format!("data: {line}\n\n")))
    })
}

/// Opens an SSE response.
fn sse(stream: GenerationStream) -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/event-stream")
        // Proxies that buffer would defeat the point of streaming entirely.
        .insert_header(("cache-control", "no-cache, no-transform"))
        .insert_header(("x-accel-buffering", "no"))
        .streaming(to_frames(stream))
}

/// Read a job posting into fields
///
/// Returns typed fields the capture form fills directly, constrained by schema
/// so a malformed generation fails loudly rather than half-populating a screen.
///
/// Send `text` — pasting is the primary path. `url` is a shortcut that usually
/// fails, because most job boards block automated fetches; when it does you get
/// `AI_FETCH_FAILED` and should paste instead. It is not retried, because you
/// are one paste away and waiting helps nobody.
#[utoipa::path(
    post,
    path = "/api/ai/extract-job",
    tag = "ai",
    request_body = ExtractJobRequest,
    responses(
        (status = 200, description = "The posting's fields", body = inline(SuccessResponse<ExtractedJob>)),
        (status = 400, description = "Neither text nor url", body = ErrorResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Email not verified", body = ErrorResponse),
        (status = 422, description = "The URL could not be fetched, or the model declined", body = ErrorResponse),
        (status = 429, description = "Generation allowance spent", body = ErrorResponse),
        (status = 502, description = "The provider failed", body = ErrorResponse),
        (status = 503, description = "Generation is not configured here", body = ErrorResponse),
    ),
    security(("BearerAuth" = []))
)]
#[post("/api/ai/extract-job")]
pub async fn extract_job_handler(
    user: VerifiedUser,
    body: web::Json<ExtractJobRequest>,
    data: web::Data<AppState>,
) -> impl Responder {
    let b = body.into_inner();

    match data
        .ai
        .extract_job
        .execute(
            UserId::from(user.user_id),
            ExtractJobInput {
                text: b.text,
                url: b.url,
            },
        )
        .await
    {
        Ok(job) => ApiResponse::success(job),
        Err(e) => map_error(e),
    }
}

/// Suggest how a CV could better answer a job
///
/// **Streams.** `text/event-stream`, one JSON object per `data:` line:
/// `{"type":"delta","text":"…"}`, then `{"type":"done", …}` with the token
/// counts.
///
/// An error can arrive **after** text has — the model may begin and then
/// decline — so failures are `{"type":"error"}` frames rather than a status
/// code. By the time one happens the 200 is long sent. A client must decide
/// what to do with text it has already shown; it must not assume an error
/// means nothing was displayed.
#[utoipa::path(
    post,
    path = "/api/ai/tailor",
    tag = "ai",
    request_body = DraftingRequest,
    responses(
        (status = 200, description = "An event stream of delta, done and error frames", body = String),
        (status = 400, description = "No CV to work from", body = ErrorResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Email not verified", body = ErrorResponse),
        (status = 429, description = "Generation allowance spent", body = ErrorResponse),
        (status = 503, description = "Generation is not configured here", body = ErrorResponse),
    ),
    security(("BearerAuth" = []))
)]
#[post("/api/ai/tailor")]
pub async fn tailor_handler(
    user: VerifiedUser,
    body: web::Json<DraftingRequest>,
    data: web::Data<AppState>,
) -> impl Responder {
    let b = body.into_inner();

    match data
        .ai
        .tailor
        .execute(
            UserId::from(user.user_id),
            DraftingInput {
                application_id: b.application_id,
                cv_id: b.cv_id,
                instruction: b.instruction,
                language: b.language,
            },
        )
        .await
    {
        // Everything that can fail before the first byte does so here, with a
        // real status code — an exhausted allowance is a 429, not a frame.
        Ok(stream) => sse(stream),
        Err(e) => map_error(e),
    }
}

/// Draft a cover letter
///
/// **Streams**, with the same frames and the same mid-stream error rule as
/// `/api/ai/tailor`.
///
/// An existing letter on the application is given to the model to revise
/// rather than replaced blind. The language is taken from the request, never
/// inferred from what is already written.
#[utoipa::path(
    post,
    path = "/api/ai/cover-letter",
    tag = "ai",
    request_body = DraftingRequest,
    responses(
        (status = 200, description = "An event stream of delta, done and error frames", body = String),
        (status = 400, description = "No CV to work from", body = ErrorResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Email not verified", body = ErrorResponse),
        (status = 429, description = "Generation allowance spent", body = ErrorResponse),
        (status = 503, description = "Generation is not configured here", body = ErrorResponse),
    ),
    security(("BearerAuth" = []))
)]
#[post("/api/ai/cover-letter")]
pub async fn cover_letter_handler(
    user: VerifiedUser,
    body: web::Json<DraftingRequest>,
    data: web::Data<AppState>,
) -> impl Responder {
    let b = body.into_inner();

    match data
        .ai
        .cover_letter
        .execute(
            UserId::from(user.user_id),
            DraftingInput {
                application_id: b.application_id,
                cv_id: b.cv_id,
                instruction: b.instruction,
                language: b.language,
            },
        )
        .await
    {
        Ok(stream) => sse(stream),
        Err(e) => map_error(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::application::ports::incoming::use_cases::{ExtractJobUseCase, TailorUseCase};
    use crate::ai::application::ports::outgoing::{GenerationError, Usage};
    use crate::ai::domain::quota::QuotaState;
    use crate::auth::adapter::outgoing::jwt::{JwtConfig, JwtTokenService};
    use crate::auth::application::ports::outgoing::token_provider::TokenProvider;
    use crate::tests::support::app_state_builder::TestAppStateBuilder;
    use actix_web::{http::StatusCode, test, App};
    use async_trait::async_trait;
    use serde_json::Value;
    use std::sync::Mutex;

    /// These routes are all behind the verified-user extractor, so every
    /// request needs a real token.
    fn signer() -> JwtTokenService {
        JwtTokenService::new(JwtConfig {
            issuer: "Lotion".to_string(),
            secret_key: "test_secret_key_for_testing_purposes_only".to_string(),
            access_token_expiry: 3600,
            refresh_token_expiry: 86400,
            verification_token_expiry: 86400,
            password_reset_expiry: 3600,
        })
    }

    struct StubExtract(Mutex<Option<Result<ExtractedJob, AiError>>>);

    #[async_trait]
    impl ExtractJobUseCase for StubExtract {
        async fn execute(&self, _o: UserId, _i: ExtractJobInput) -> Result<ExtractedJob, AiError> {
            self.0.lock().unwrap().take().unwrap()
        }
    }

    struct StubTailor(Mutex<Option<Vec<Result<GenerationEvent, GenerationError>>>>);

    #[async_trait]
    impl TailorUseCase for StubTailor {
        async fn execute(
            &self,
            _o: UserId,
            _i: DraftingInput,
        ) -> Result<GenerationStream, AiError> {
            let events = self.0.lock().unwrap().take().unwrap();
            Ok(Box::pin(futures::stream::iter(events)))
        }
    }

    async fn extract(result: Result<ExtractedJob, AiError>) -> (StatusCode, Value) {
        let state = TestAppStateBuilder::default()
            .with_extract_job(StubExtract(Mutex::new(Some(result))))
            .build();

        let j = signer();
        let token = j.generate_access_token(Uuid::new_v4(), true).unwrap();
        let provider: std::sync::Arc<dyn TokenProvider + Send + Sync> = std::sync::Arc::new(j);

        let app = test::init_service(
            App::new()
                .app_data(state)
                .app_data(web::Data::new(provider))
                .service(extract_job_handler),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/ai/extract-job")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(serde_json::json!({ "text": "a posting" }))
            .to_request();

        let resp = test::call_service(&app, req).await;
        let status = resp.status();
        (status, test::read_body_json(resp).await)
    }

    async fn stream_body(
        events: Vec<Result<GenerationEvent, GenerationError>>,
    ) -> (StatusCode, String) {
        let state = TestAppStateBuilder::default()
            .with_tailor(StubTailor(Mutex::new(Some(events))))
            .build();

        let j = signer();
        let token = j.generate_access_token(Uuid::new_v4(), true).unwrap();
        let provider: std::sync::Arc<dyn TokenProvider + Send + Sync> = std::sync::Arc::new(j);

        let app = test::init_service(
            App::new()
                .app_data(state)
                .app_data(web::Data::new(provider))
                .service(tailor_handler),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/ai/tailor")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(serde_json::json!({ "application_id": Uuid::new_v4() }))
            .to_request();

        let resp = test::call_service(&app, req).await;
        let status = resp.status();
        let body = String::from_utf8(test::read_body(resp).await.to_vec()).unwrap();
        (status, body)
    }

    /// The default builder leaves generation switched off, which is what a
    /// deployment without credentials looks like.
    async fn stream_disabled() -> (StatusCode, String) {
        let state = TestAppStateBuilder::default().build();
        let j = signer();
        let token = j.generate_access_token(Uuid::new_v4(), true).unwrap();
        let provider: std::sync::Arc<dyn TokenProvider + Send + Sync> = std::sync::Arc::new(j);

        let app = test::init_service(
            App::new()
                .app_data(state)
                .app_data(web::Data::new(provider))
                .service(tailor_handler),
        )
        .await;

        let resp = test::call_service(
            &app,
            test::TestRequest::post()
                .uri("/api/ai/tailor")
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(serde_json::json!({ "application_id": Uuid::new_v4() }))
                .to_request(),
        )
        .await;

        let status = resp.status();
        (
            status,
            String::from_utf8(test::read_body(resp).await.to_vec()).unwrap(),
        )
    }

    async fn stream_headers() -> (String, String) {
        let state = TestAppStateBuilder::default()
            .with_tailor(StubTailor(Mutex::new(Some(vec![Ok(
                GenerationEvent::Delta("hi".into()),
            )]))))
            .build();

        let j = signer();
        let token = j.generate_access_token(Uuid::new_v4(), true).unwrap();
        let provider: std::sync::Arc<dyn TokenProvider + Send + Sync> = std::sync::Arc::new(j);

        let app = test::init_service(
            App::new()
                .app_data(state)
                .app_data(web::Data::new(provider))
                .service(tailor_handler),
        )
        .await;

        let resp = test::call_service(
            &app,
            test::TestRequest::post()
                .uri("/api/ai/tailor")
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(serde_json::json!({ "application_id": Uuid::new_v4() }))
                .to_request(),
        )
        .await;

        (
            resp.headers()
                .get("content-type")
                .unwrap()
                .to_str()
                .unwrap()
                .to_string(),
            resp.headers()
                .get("x-accel-buffering")
                .unwrap()
                .to_str()
                .unwrap()
                .to_string(),
        )
    }

    // ── extract-job ────────────────────────────────────────────────────

    #[actix_web::test]
    async fn an_extracted_posting_comes_back_as_fields() {
        let (status, body) = extract(Ok(ExtractedJob {
            title: "Backend Engineer".into(),
            company: "Acme".into(),
            required_skills: vec!["Rust".into()],
            ..Default::default()
        }))
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["data"]["title"], "Backend Engineer");
        assert_eq!(body["data"]["required_skills"][0], "Rust");
    }

    /// The four codes the frontend maps to a sentence and a recovery. Each has
    /// to be distinguishable — a generic failure would leave every one of them
    /// rendering the same unhelpful message.
    #[actix_web::test]
    async fn each_failure_carries_its_own_code() {
        let cases = [
            (
                AiError::Disabled,
                StatusCode::SERVICE_UNAVAILABLE,
                "AI_DISABLED",
            ),
            (
                AiError::QuotaExceeded(Box::new(QuotaState {
                    used: 5,
                    limit: Some(5),
                    resets_at: chrono::Utc::now(),
                })),
                StatusCode::TOO_MANY_REQUESTS,
                "AI_QUOTA_EXCEEDED",
            ),
            (
                AiError::FetchFailed("403".into()),
                StatusCode::UNPROCESSABLE_ENTITY,
                "AI_FETCH_FAILED",
            ),
            (
                AiError::Refused("cyber".into()),
                StatusCode::UNPROCESSABLE_ENTITY,
                "AI_REFUSED",
            ),
            (AiError::Timeout, StatusCode::GATEWAY_TIMEOUT, "AI_TIMEOUT"),
            (
                AiError::Upstream("boom".into()),
                StatusCode::BAD_GATEWAY,
                "AI_UPSTREAM_ERROR",
            ),
        ];

        for (error, expected_status, expected_code) in cases {
            let (status, body) = extract(Err(error)).await;

            assert_eq!(status, expected_status, "for {expected_code}");
            assert_eq!(body["error"]["code"], expected_code);
        }
    }

    /// A provider's message can carry prompt fragments, and a prompt here
    /// contains somebody's CV. It is logged, never returned.
    #[actix_web::test]
    async fn a_provider_message_is_not_echoed_to_the_client() {
        let (_, body) = extract(Err(AiError::Upstream(
            "prompt was: Jane Doe, 12 Bridge Street".into(),
        )))
        .await;

        let rendered = body.to_string();
        assert!(
            !rendered.contains("Bridge Street"),
            "provider detail must not reach the client: {rendered}"
        );
    }

    /// The quota refusal says when it lifts, so the UI can say something
    /// better than "try later".
    #[actix_web::test]
    async fn the_quota_refusal_says_when_it_resets() {
        let (_, body) = extract(Err(AiError::QuotaExceeded(Box::new(QuotaState {
            used: 5,
            limit: Some(5),
            resets_at: chrono::Utc::now(),
        }))))
        .await;

        assert!(body["error"]["message"]
            .as_str()
            .unwrap()
            .contains("resets at"));
    }

    // ── streaming ──────────────────────────────────────────────────────

    #[actix_web::test]
    async fn a_stream_is_served_as_events_not_json() {
        let (status, body) = stream_body(vec![
            Ok(GenerationEvent::Delta("Hel".into())),
            Ok(GenerationEvent::Delta("lo".into())),
            Ok(GenerationEvent::Completed(Usage {
                input_tokens: 10,
                output_tokens: 2,
                cached_input_tokens: 900,
            })),
        ])
        .await;

        assert_eq!(status, StatusCode::OK);
        assert!(body.contains(r#"data: {"type":"delta","text":"Hel"}"#));
        assert!(body.contains(r#""type":"done""#));
        assert!(
            body.contains(r#""cached_input_tokens":900"#),
            "the cache figure must reach the client: it is how anyone notices caching has stopped"
        );
    }

    /// The case that decides whether a client is written correctly. By the
    /// time a refusal arrives the 200 is long sent and text is on screen, so
    /// it has to be a frame — a stream that simply stopped would be
    /// indistinguishable from a finished generation.
    #[actix_web::test]
    async fn a_mid_stream_refusal_arrives_as_a_frame_after_the_text() {
        let (status, body) = stream_body(vec![
            Ok(GenerationEvent::Delta("Sure, I can".into())),
            Err(GenerationError::Refused("cyber".into())),
        ])
        .await;

        assert_eq!(
            status,
            StatusCode::OK,
            "the status was sent before the refusal"
        );

        let lines: Vec<&str> = body.lines().filter(|l| l.starts_with("data:")).collect();
        assert!(lines[0].contains(r#""type":"delta""#));
        assert!(lines[1].contains(r#""type":"error""#));
        assert!(lines[1].contains("AI_REFUSED"));
    }

    /// Anything that can fail before the first byte still gets a real status
    /// code — an exhausted allowance is a 429, not a frame nobody checks.
    #[actix_web::test]
    async fn a_failure_before_the_stream_opens_is_a_status_not_a_frame() {
        let (status, _) = stream_disabled().await;

        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    }

    #[actix_web::test]
    async fn a_stream_is_marked_unbuffered_so_proxies_do_not_defeat_it() {
        let headers = stream_headers().await;

        assert_eq!(headers.0, "text/event-stream");
        assert_eq!(headers.1, "no");
    }
}
