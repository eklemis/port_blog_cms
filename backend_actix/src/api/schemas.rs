// src/api/schemas.rs
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

#[derive(Serialize, ToSchema)]
pub struct ErrorDetail {
    /// Error code for programmatic handling
    #[schema(example = "INVALID_FILE_NAME")]
    pub code: String,

    /// Human-readable error message
    #[schema(example = "Invalid file name")]
    pub message: String,
}
