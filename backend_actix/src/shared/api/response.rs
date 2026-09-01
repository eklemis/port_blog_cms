// src/shared/api/response.rs
use actix_web::{http::StatusCode, HttpResponse};

use crate::shared::api::ErrorCode;
use serde::Serialize;

#[derive(Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ApiError>,
}

#[derive(Serialize, Clone)]
pub struct ApiError {
    pub code: ErrorCode,
    pub message: String,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn success(data: T) -> HttpResponse {
        HttpResponse::Ok().json(ApiResponse {
            success: true,
            data: Some(data),
            error: None,
        })
    }

    pub fn created(data: T) -> HttpResponse {
        HttpResponse::Created().json(ApiResponse {
            success: true,
            data: Some(data),
            error: None,
        })
    }
}

impl ApiResponse<()> {
    pub fn no_content() -> HttpResponse {
        HttpResponse::NoContent().finish()
    }

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

    pub fn not_found(code: ErrorCode, message: &str) -> HttpResponse {
        Self::error(StatusCode::NOT_FOUND, code, message)
    }

    pub fn bad_request(code: ErrorCode, message: &str) -> HttpResponse {
        Self::error(StatusCode::BAD_REQUEST, code, message)
    }

    pub fn forbidden(code: ErrorCode, message: &str) -> HttpResponse {
        Self::error(StatusCode::FORBIDDEN, code, message)
    }

    pub fn unauthorized(code: ErrorCode, message: &str) -> HttpResponse {
        Self::error(StatusCode::UNAUTHORIZED, code, message)
    }

    pub fn conflict(code: ErrorCode, message: &str) -> HttpResponse {
        Self::error(StatusCode::CONFLICT, code, message)
    }

    pub fn internal_error() -> HttpResponse {
        Self::error(
            StatusCode::INTERNAL_SERVER_ERROR,
            ErrorCode::InternalError,
            "An unexpected error occurred",
        )
    }
}
