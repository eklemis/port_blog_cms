// src/shared/api/response.rs
use actix_web::{http::StatusCode, HttpResponse};

use crate::shared::api::ErrorCode;
use serde::Serialize;

#[derive(Serialize)]
/// The envelope every endpoint returns, success or failure.
///
/// Exactly one of `data` and `error` is present; both are omitted when they
/// are `None`, so a success carries no `error` key at all.
pub struct ApiResponse<T: Serialize> {
    /// True on success, false on failure. Redundant with the presence of
    /// `error`, and kept because clients branch on it.
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// The payload. Present only on success.
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// The failure detail. Present only on failure.
    pub error: Option<ApiError>,
}

#[derive(Serialize, Clone)]
/// The failure detail carried in [`ApiResponse::error`].
pub struct ApiError {
    /// The stable, machine-readable code. **Branch on this**, not on
    /// `message`. See `docs/API_ERRORS.md`.
    pub code: ErrorCode,
    /// Human-readable prose. May change without notice; not a contract.
    pub message: String,
}

impl<T: Serialize> ApiResponse<T> {
    /// 200 with a payload.
    pub fn success(data: T) -> HttpResponse {
        HttpResponse::Ok().json(ApiResponse {
            success: true,
            data: Some(data),
            error: None,
        })
    }

    /// 201 with the created resource.
    pub fn created(data: T) -> HttpResponse {
        HttpResponse::Created().json(ApiResponse {
            success: true,
            data: Some(data),
            error: None,
        })
    }
}

impl ApiResponse<()> {
    /// 204 with an empty body. Carries no envelope, because there is nothing
    /// to wrap.
    pub fn no_content() -> HttpResponse {
        HttpResponse::NoContent().finish()
    }

    /// A failure at an arbitrary status.
    ///
    /// The status is chosen by the caller rather than derived from `code`,
    /// because three codes are legitimately returned with two different
    /// statuses. See the [`ErrorCode`] module documentation.
    pub fn error(status: StatusCode, code: ErrorCode, message: &str) -> HttpResponse {
        HttpResponse::build(status).json(ApiResponse::<()> {
            success: false,
            data: None,
            error: Some(ApiError {
                code,
                message: message.to_string(),
            }),
        })
    }

    /// 404.
    pub fn not_found(code: ErrorCode, message: &str) -> HttpResponse {
        Self::error(StatusCode::NOT_FOUND, code, message)
    }

    /// 400.
    pub fn bad_request(code: ErrorCode, message: &str) -> HttpResponse {
        Self::error(StatusCode::BAD_REQUEST, code, message)
    }

    /// 403 — authenticated, but not allowed.
    pub fn forbidden(code: ErrorCode, message: &str) -> HttpResponse {
        Self::error(StatusCode::FORBIDDEN, code, message)
    }

    /// 401 — not authenticated, or the credential was rejected.
    pub fn unauthorized(code: ErrorCode, message: &str) -> HttpResponse {
        Self::error(StatusCode::UNAUTHORIZED, code, message)
    }

    /// 409.
    pub fn conflict(code: ErrorCode, message: &str) -> HttpResponse {
        Self::error(StatusCode::CONFLICT, code, message)
    }

    /// 500 with a deliberately generic message. Details are logged, never
    /// returned.
    pub fn internal_error() -> HttpResponse {
        Self::error(
            StatusCode::INTERNAL_SERVER_ERROR,
            ErrorCode::InternalError,
            "An unexpected error occurred",
        )
    }
}
