#[cfg(test)]
pub mod test_helpers {
    use crate::auth::application::services::jwt::{JwtConfig, JwtService};

    pub fn create_test_jwt_service() -> JwtService {
        let jwt_config = JwtConfig {
            issuer: "Ekstion".to_string(),
            secret_key: "test_secret_key_for_testing_only".to_string(),
            access_token_expiry: 3600,
            refresh_token_expiry: 86400,
            verification_token_expiry: 86400,
        };
        JwtService::new(jwt_config)
    }
}
