// src/shared/api/json_config.rs
use crate::shared::api::ApiResponse;
use actix_web::web::JsonConfig;

pub fn custom_json_config() -> JsonConfig {
    JsonConfig::default().error_handler(|err, _req| {
        let message = err.to_string();
        actix_web::error::InternalError::from_response(
            err,
            ApiResponse::bad_request("VALIDATION_ERROR", &message),
        )
        .into()
    })
}
