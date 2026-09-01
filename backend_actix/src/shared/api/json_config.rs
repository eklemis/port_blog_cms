// src/shared/api/json_config.rs
use crate::shared::api::{ApiResponse, ErrorCode};
use actix_web::web::JsonConfig;

/// Makes a malformed JSON body answer in the same envelope as everything else.
///
/// Without this, Actix's default returns a bare text body, so a client would
/// have to special-case parse failures.
pub fn custom_json_config() -> JsonConfig {
    JsonConfig::default().error_handler(|err, _req| {
        let message = err.to_string();
        actix_web::error::InternalError::from_response(
            err,
            ApiResponse::bad_request(ErrorCode::ValidationError, &message),
        )
        .into()
    })
}
