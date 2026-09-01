// src/api/schemas.rs
use crate::shared::api::ErrorCode;
use serde::Serialize;
use utoipa::ToSchema;

/// Standard success response wrapper
#[derive(Serialize, ToSchema)]
#[serde(bound = "T: Serialize")]
pub struct SuccessResponse<T> {
    /// Always true for successful responses
    #[schema(example = true)]
    pub success: bool,
    /// Response data
    pub data: T,
}

/// Standard error response wrapper
#[derive(Serialize, ToSchema)]
pub struct ErrorResponse {
    /// Always false for error responses
    #[schema(example = false)]
    pub success: bool,
    /// Error details
    pub error: ErrorDetail,
}

/// The `error` object inside a failed response.
#[derive(Serialize, ToSchema)]
pub struct ErrorDetail {
    /// Machine-readable error code. This is the stable contract — branch on it
    /// rather than on `message`, which is prose and may change.
    pub code: ErrorCode,

    /// Human-readable error message
    #[schema(example = "Invalid file name")]
    pub message: String,
}
