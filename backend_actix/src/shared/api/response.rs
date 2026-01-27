// src/shared/api/response.rs
use actix_web::{http::StatusCode, HttpResponse};
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
    pub code: String,
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

    pub fn error(status: StatusCode, code: &str, message: &str) -> HttpResponse {
        HttpResponse::build(status).json(ApiResponse::<()> {
            success: false,
            data: None,
            error: Some(ApiError {
                code: code.to_string(),
                message: message.to_string(),
            }),
        })
    }

    pub fn not_found(code: &str, message: &str) -> HttpResponse {
        Self::error(StatusCode::NOT_FOUND, code, message)
    }

    pub fn bad_request(code: &str, message: &str) -> HttpResponse {
        Self::error(StatusCode::BAD_REQUEST, code, message)
    }

    pub fn forbidden(code: &str, message: &str) -> HttpResponse {
        Self::error(StatusCode::FORBIDDEN, code, message)
    }

    pub fn unauthorized(code: &str, message: &str) -> HttpResponse {
        Self::error(StatusCode::UNAUTHORIZED, code, message)
    }

    pub fn conflict(code: &str, message: &str) -> HttpResponse {
        Self::error(StatusCode::CONFLICT, code, message)
    }

    pub fn internal_error() -> HttpResponse {
        Self::error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "INTERNAL_ERROR",
            "An unexpected error occurred",
        )
    }
}
